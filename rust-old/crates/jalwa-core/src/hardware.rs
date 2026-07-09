//! Hardware device management — USB storage auto-detect, optical disc browsing,
//! hotplug events, and graceful device removal.
//!
//! Wraps [`yukti`] to provide media-player-specific device handling:
//! - USB storage: auto-add mount point as scan path on attach
//! - Optical drives: read TOC, detect CDDA/DVD/Blu-ray
//! - Hotplug: subscribe to device events, notify the UI layer
//! - Safe eject: stop playback from a device before removal

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use tracing::{debug, error, info, warn};
use yukti::device::{DeviceClass, DeviceId, DeviceInfo, DeviceState};
use yukti::event::{DeviceEvent, DeviceEventKind};

#[cfg(feature = "yukti")]
pub use yukti::optical::{DiscToc, DiscType, TocEntry, TrackType, TrayState};

/// Media-player-relevant device events emitted to the UI layer.
#[derive(Debug, Clone)]
pub enum HardwareEvent {
    /// A USB storage device was mounted — the mount point can be scanned.
    UsbMounted {
        device_id: DeviceId,
        label: String,
        mount_point: PathBuf,
    },
    /// A USB storage device was removed.
    UsbRemoved { device_id: DeviceId, label: String },
    /// An optical disc was inserted (CDDA, DVD, Blu-ray).
    DiscInserted {
        device_id: DeviceId,
        disc_type: yukti::optical::DiscType,
        dev_path: PathBuf,
    },
    /// An optical disc was ejected.
    DiscEjected { device_id: DeviceId },
    /// A device that is currently being played from was removed.
    PlaybackDeviceRemoved {
        device_id: DeviceId,
        mount_point: PathBuf,
    },
    /// Device error (IO failure, permission denied, etc).
    DeviceError {
        device_id: DeviceId,
        message: String,
    },
}

/// Manages hardware devices relevant to the media player.
///
/// Wraps yukti's `LinuxDeviceManager` and translates raw device events
/// into media-player-specific [`HardwareEvent`]s.
pub struct HardwareManager {
    /// Known devices (keyed by DeviceId).
    devices: HashMap<DeviceId, DeviceInfo>,
    /// Paths currently being played from (mount points or dev paths).
    active_playback_paths: Vec<PathBuf>,
    /// Channel to send events to the UI/app layer.
    event_tx: mpsc::Sender<HardwareEvent>,
    /// Receiver for raw yukti device events (from udev monitor).
    #[cfg(target_os = "linux")]
    yukti_rx: Option<mpsc::Receiver<DeviceEvent>>,
    /// The underlying yukti device manager.
    #[cfg(target_os = "linux")]
    manager: yukti::LinuxDeviceManager,
}

impl HardwareManager {
    /// Create a new hardware manager. Returns the manager and the event receiver
    /// for the app layer to poll.
    pub fn new() -> (Self, mpsc::Receiver<HardwareEvent>) {
        let (event_tx, event_rx) = mpsc::channel();

        #[cfg(target_os = "linux")]
        let manager = yukti::LinuxDeviceManager::new();

        let hw = Self {
            devices: HashMap::new(),
            active_playback_paths: Vec::new(),
            event_tx,
            #[cfg(target_os = "linux")]
            yukti_rx: None,
            #[cfg(target_os = "linux")]
            manager,
        };

        (hw, event_rx)
    }

    /// Start monitoring for device hotplug events.
    ///
    /// Call this once at startup. After this, poll with [`poll()`](Self::poll).
    #[cfg(target_os = "linux")]
    pub fn start_monitoring(&mut self) -> crate::Result<()> {
        info!("starting hardware device monitoring");
        let rx = self
            .manager
            .start_monitor()
            .map_err(|e| crate::JalwaError::Library(format!("yukti monitor: {e}")))?;
        self.yukti_rx = Some(rx);

        // Initial enumeration
        self.enumerate()?;

        Ok(())
    }

    /// Stop the hotplug monitor.
    #[cfg(target_os = "linux")]
    pub fn stop_monitoring(&self) {
        info!("stopping hardware device monitoring");
        self.manager.stop_monitor();
    }

    /// Enumerate currently connected devices and emit events for relevant ones.
    #[cfg(target_os = "linux")]
    pub fn enumerate(&mut self) -> crate::Result<()> {
        use yukti::device::Device;

        let devices = self
            .manager
            .enumerate()
            .map_err(|e| crate::JalwaError::Library(format!("device enumerate: {e}")))?;

        for info in devices {
            if is_media_relevant(&info) {
                debug!(device = %info.id, class = %info.class, "discovered device");
                self.handle_device_info(info);
            }
        }
        Ok(())
    }

    /// Poll for new device events. Call this periodically (e.g. in the GUI update loop).
    /// Returns the number of events processed.
    #[cfg(target_os = "linux")]
    pub fn poll(&mut self) -> usize {
        let Some(ref rx) = self.yukti_rx else {
            return 0;
        };

        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }

        let count = events.len();
        for event in events {
            self.handle_yukti_event(event);
        }
        count
    }

    /// Register a path as actively being played from. If the device at this
    /// path is removed, a [`HardwareEvent::PlaybackDeviceRemoved`] will be emitted.
    pub fn register_playback_path(&mut self, path: PathBuf) {
        if !self.active_playback_paths.contains(&path) {
            debug!(path = %path.display(), "registered playback path");
            self.active_playback_paths.push(path);
        }
    }

    /// Unregister a playback path (e.g. when playback stops).
    pub fn unregister_playback_path(&mut self, path: &Path) {
        self.active_playback_paths.retain(|p| p != path);
    }

    /// Get all currently known media-relevant devices.
    pub fn devices(&self) -> Vec<&DeviceInfo> {
        self.devices.values().collect()
    }

    /// Get USB storage devices that are currently mounted.
    pub fn mounted_usb_devices(&self) -> Vec<&DeviceInfo> {
        self.devices
            .values()
            .filter(|d| d.class == DeviceClass::UsbStorage && d.is_mounted())
            .collect()
    }

    /// Get optical drives.
    pub fn optical_drives(&self) -> Vec<&DeviceInfo> {
        self.devices
            .values()
            .filter(|d| d.class == DeviceClass::Optical)
            .collect()
    }

    /// Safely eject a device — stops playback if needed, then ejects.
    pub fn safe_eject(&mut self, id: &DeviceId) -> crate::Result<()> {
        let info = self
            .devices
            .get(id)
            .ok_or_else(|| crate::JalwaError::NotFound(format!("device {id}")))?;

        // Check if we're playing from this device
        if let Some(mount) = &info.mount_point
            && self
                .active_playback_paths
                .iter()
                .any(|p| p.starts_with(mount))
        {
            info!(device = %id, "stopping playback before eject");
            let _ = self.event_tx.send(HardwareEvent::PlaybackDeviceRemoved {
                device_id: id.clone(),
                mount_point: mount.clone(),
            });
        }

        info!(device = %id, "ejecting device");
        // Delegate to yukti for the actual eject
        #[cfg(target_os = "linux")]
        {
            self.manager
                .eject(id)
                .map_err(|e| crate::JalwaError::Library(format!("eject: {e}")))?;
        }

        Ok(())
    }

    /// Read the table of contents from an optical drive.
    #[cfg(target_os = "linux")]
    pub fn read_disc_toc(&self, dev_path: &Path) -> crate::Result<DiscToc> {
        yukti::optical::read_toc(dev_path)
            .map_err(|e| crate::JalwaError::Library(format!("read TOC: {e}")))
    }

    /// Open/eject an optical drive tray.
    #[cfg(target_os = "linux")]
    pub fn open_tray(&self, dev_path: &Path) -> crate::Result<()> {
        yukti::optical::open_tray(dev_path)
            .map_err(|e| crate::JalwaError::Library(format!("open tray: {e}")))
    }

    /// Close an optical drive tray.
    #[cfg(target_os = "linux")]
    pub fn close_tray(&self, dev_path: &Path) -> crate::Result<()> {
        yukti::optical::close_tray(dev_path)
            .map_err(|e| crate::JalwaError::Library(format!("close tray: {e}")))
    }

    /// Query optical drive status.
    #[cfg(target_os = "linux")]
    pub fn tray_state(&self, dev_path: &Path) -> crate::Result<TrayState> {
        yukti::optical::drive_status(dev_path)
            .map_err(|e| crate::JalwaError::Library(format!("drive status: {e}")))
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Process a single yukti device event. Public for benchmarking.
    #[doc(hidden)]
    pub fn handle_yukti_event(&mut self, event: DeviceEvent) {
        let id = &event.device_id;
        let class = event.device_class;

        if !is_class_relevant(class) {
            return;
        }

        match &event.kind {
            DeviceEventKind::Attached => {
                info!(device = %id, class = %class, "device attached");
                if let Some(info) = event.device_info.clone() {
                    self.handle_device_info(info);
                }
            }
            DeviceEventKind::Detached => {
                info!(device = %id, class = %class, "device detached");
                self.handle_detach(id, class);
            }
            DeviceEventKind::Mounted { mount_point } => {
                info!(device = %id, mount = %mount_point.display(), "device mounted");
                self.handle_mount(id, mount_point.clone());
            }
            DeviceEventKind::Unmounted => {
                info!(device = %id, "device unmounted");
                if let Some(info) = self.devices.get_mut(id) {
                    info.state = DeviceState::Ready;
                    info.mount_point = None;
                }
            }
            DeviceEventKind::MediaChanged => {
                info!(device = %id, "media changed (new disc?)");
                if class == DeviceClass::Optical {
                    self.handle_optical_change(id);
                }
            }
            DeviceEventKind::Error { message } => {
                error!(device = %id, error = %message, "device error");
                let _ = self.event_tx.send(HardwareEvent::DeviceError {
                    device_id: id.clone(),
                    message: message.clone(),
                });
            }
            _ => {
                debug!(device = %id, kind = %event.kind, "unhandled device event kind");
            }
        }
    }

    fn handle_device_info(&mut self, info: DeviceInfo) {
        let id = info.id.clone();

        match info.class {
            DeviceClass::UsbStorage => {
                if info.is_mounted()
                    && let Some(mount) = &info.mount_point
                {
                    let _ = self.event_tx.send(HardwareEvent::UsbMounted {
                        device_id: id.clone(),
                        label: info.display_name().into_owned(),
                        mount_point: mount.clone(),
                    });
                }
            }
            DeviceClass::Optical => {
                if info.state != DeviceState::NoMedia {
                    let disc_type = detect_optical_type(&info);
                    let _ = self.event_tx.send(HardwareEvent::DiscInserted {
                        device_id: id.clone(),
                        disc_type,
                        dev_path: info.dev_path.clone(),
                    });
                }
            }
            _ => {}
        }

        self.devices.insert(id, info);
    }

    fn handle_mount(&mut self, id: &DeviceId, mount_point: PathBuf) {
        if let Some(info) = self.devices.get_mut(id) {
            info.state = DeviceState::Mounted;
            info.mount_point = Some(mount_point.clone());

            if info.class == DeviceClass::UsbStorage {
                let _ = self.event_tx.send(HardwareEvent::UsbMounted {
                    device_id: id.clone(),
                    label: info.display_name().into_owned(),
                    mount_point,
                });
            }
        }
    }

    fn handle_detach(&mut self, id: &DeviceId, class: DeviceClass) {
        // Check if we're playing from this device
        if let Some(info) = self.devices.get(id)
            && let Some(mount) = &info.mount_point
            && self
                .active_playback_paths
                .iter()
                .any(|p| p.starts_with(mount))
        {
            warn!(device = %id, "device removed while playing — notifying app");
            let _ = self.event_tx.send(HardwareEvent::PlaybackDeviceRemoved {
                device_id: id.clone(),
                mount_point: mount.clone(),
            });
        }

        match class {
            DeviceClass::UsbStorage => {
                let label = self
                    .devices
                    .get(id)
                    .map(|d| d.display_name().into_owned())
                    .unwrap_or_else(|| id.to_string());
                let _ = self.event_tx.send(HardwareEvent::UsbRemoved {
                    device_id: id.clone(),
                    label,
                });
            }
            DeviceClass::Optical => {
                let _ = self.event_tx.send(HardwareEvent::DiscEjected {
                    device_id: id.clone(),
                });
            }
            _ => {}
        }

        self.devices.remove(id);
    }

    fn handle_optical_change(&mut self, id: &DeviceId) {
        if let Some(info) = self.devices.get(id) {
            let disc_type = detect_optical_type(info);
            let _ = self.event_tx.send(HardwareEvent::DiscInserted {
                device_id: id.clone(),
                disc_type,
                dev_path: info.dev_path.clone(),
            });
        }
    }
}

/// Check if a device class is relevant to a media player.
fn is_class_relevant(class: DeviceClass) -> bool {
    matches!(
        class,
        DeviceClass::UsbStorage | DeviceClass::Optical | DeviceClass::SdCard
    )
}

/// Check if a specific device is relevant (class + not internal).
fn is_media_relevant(info: &DeviceInfo) -> bool {
    is_class_relevant(info.class)
}

/// Detect optical disc type from device info and filesystem.
///
/// Maps filesystem types to disc types since yukti's `detect_disc_type`
/// expects media type strings (e.g. "cd", "dvd") not filesystem names.
fn detect_optical_type(info: &DeviceInfo) -> yukti::optical::DiscType {
    match info.fs_type.as_deref() {
        Some("iso9660") => yukti::optical::DiscType::CdData,
        Some("udf") => yukti::optical::DiscType::DvdRom,
        Some(other) => yukti::optical::detect_disc_type(other, false, true),
        None => yukti::optical::DiscType::Unknown,
    }
}

/// Check if a file path is on a removable device by comparing against
/// known device mount points.
pub fn is_on_removable_device(path: &Path, devices: &[&DeviceInfo]) -> Option<DeviceId> {
    for dev in devices {
        if let Some(mount) = &dev.mount_point
            && path.starts_with(mount)
        {
            return Some(dev.id.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_usb_device(id: &str, mounted: bool) -> DeviceInfo {
        let mut info = DeviceInfo::new(
            DeviceId::new(id),
            PathBuf::from(format!("/dev/{id}")),
            DeviceClass::UsbStorage,
        );
        if mounted {
            info.state = DeviceState::Mounted;
            info.mount_point = Some(PathBuf::from(format!("/mnt/{id}")));
        }
        info.label = Some("Test USB".into());
        info
    }

    fn make_optical_device(id: &str, has_media: bool) -> DeviceInfo {
        let mut info = DeviceInfo::new(
            DeviceId::new(id),
            PathBuf::from(format!("/dev/{id}")),
            DeviceClass::Optical,
        );
        if has_media {
            info.state = DeviceState::Ready;
            info.fs_type = Some("iso9660".into());
        } else {
            info.state = DeviceState::NoMedia;
        }
        info
    }

    // -----------------------------------------------------------------------
    // HardwareManager unit tests
    // -----------------------------------------------------------------------

    #[test]
    fn new_creates_empty_manager() {
        let (hw, _rx) = HardwareManager::new();
        assert!(hw.devices.is_empty());
        assert!(hw.active_playback_paths.is_empty());
    }

    #[test]
    fn register_and_unregister_playback_path() {
        let (mut hw, _rx) = HardwareManager::new();
        let path = PathBuf::from("/mnt/usb/music/song.flac");

        hw.register_playback_path(path.clone());
        assert_eq!(hw.active_playback_paths.len(), 1);

        // Duplicate registration should not add again
        hw.register_playback_path(path.clone());
        assert_eq!(hw.active_playback_paths.len(), 1);

        hw.unregister_playback_path(&path);
        assert!(hw.active_playback_paths.is_empty());
    }

    #[test]
    fn handle_device_info_usb_mounted() {
        let (mut hw, rx) = HardwareManager::new();
        let info = make_usb_device("sdb1", true);
        let id = info.id.clone();

        hw.handle_device_info(info);

        assert!(hw.devices.contains_key(&id));
        let event = rx.try_recv().unwrap();
        assert!(matches!(event, HardwareEvent::UsbMounted { .. }));
    }

    #[test]
    fn handle_device_info_usb_not_mounted() {
        let (mut hw, rx) = HardwareManager::new();
        let info = make_usb_device("sdb1", false);

        hw.handle_device_info(info);

        assert_eq!(hw.devices.len(), 1);
        // No event for unmounted USB
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn handle_device_info_optical_with_media() {
        let (mut hw, rx) = HardwareManager::new();
        let info = make_optical_device("sr0", true);

        hw.handle_device_info(info);

        let event = rx.try_recv().unwrap();
        assert!(matches!(event, HardwareEvent::DiscInserted { .. }));
    }

    #[test]
    fn handle_device_info_optical_no_media() {
        let (mut hw, rx) = HardwareManager::new();
        let info = make_optical_device("sr0", false);

        hw.handle_device_info(info);

        assert_eq!(hw.devices.len(), 1);
        // No event for empty drive
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn handle_detach_usb() {
        let (mut hw, rx) = HardwareManager::new();
        let info = make_usb_device("sdb1", true);
        let id = info.id.clone();
        hw.devices.insert(id.clone(), info);
        // Drain mount event
        let _ = rx.try_recv();

        hw.handle_detach(&id, DeviceClass::UsbStorage);

        assert!(!hw.devices.contains_key(&id));
        let event = rx.try_recv().unwrap();
        assert!(matches!(event, HardwareEvent::UsbRemoved { .. }));
    }

    #[test]
    fn handle_detach_optical() {
        let (mut hw, rx) = HardwareManager::new();
        let info = make_optical_device("sr0", true);
        let id = info.id.clone();
        hw.devices.insert(id.clone(), info);

        hw.handle_detach(&id, DeviceClass::Optical);

        assert!(!hw.devices.contains_key(&id));
        let event = rx.try_recv().unwrap();
        assert!(matches!(event, HardwareEvent::DiscEjected { .. }));
    }

    #[test]
    fn handle_detach_during_playback() {
        let (mut hw, rx) = HardwareManager::new();
        let info = make_usb_device("sdb1", true);
        let id = info.id.clone();
        hw.devices.insert(id.clone(), info);

        // Register a playback path on this device
        hw.register_playback_path(PathBuf::from("/mnt/sdb1/music/song.flac"));

        hw.handle_detach(&id, DeviceClass::UsbStorage);

        // Should get PlaybackDeviceRemoved first, then UsbRemoved
        let event1 = rx.try_recv().unwrap();
        assert!(matches!(
            event1,
            HardwareEvent::PlaybackDeviceRemoved { .. }
        ));
        let event2 = rx.try_recv().unwrap();
        assert!(matches!(event2, HardwareEvent::UsbRemoved { .. }));
    }

    #[test]
    fn handle_mount_updates_state() {
        let (mut hw, rx) = HardwareManager::new();
        let info = make_usb_device("sdb1", false);
        let id = info.id.clone();
        hw.devices.insert(id.clone(), info);

        hw.handle_mount(&id, PathBuf::from("/mnt/usb"));

        let device = hw.devices.get(&id).unwrap();
        assert_eq!(device.state, DeviceState::Mounted);
        assert_eq!(device.mount_point, Some(PathBuf::from("/mnt/usb")));

        let event = rx.try_recv().unwrap();
        assert!(matches!(event, HardwareEvent::UsbMounted { .. }));
    }

    #[test]
    fn mounted_usb_devices_filter() {
        let (mut hw, _rx) = HardwareManager::new();
        hw.devices
            .insert(DeviceId::new("a"), make_usb_device("a", true));
        hw.devices
            .insert(DeviceId::new("b"), make_usb_device("b", false));
        hw.devices
            .insert(DeviceId::new("c"), make_optical_device("c", true));

        let mounted = hw.mounted_usb_devices();
        assert_eq!(mounted.len(), 1);
        assert_eq!(mounted[0].id.as_str(), "a");
    }

    #[test]
    fn optical_drives_filter() {
        let (mut hw, _rx) = HardwareManager::new();
        hw.devices
            .insert(DeviceId::new("a"), make_usb_device("a", true));
        hw.devices
            .insert(DeviceId::new("sr0"), make_optical_device("sr0", true));
        hw.devices
            .insert(DeviceId::new("sr1"), make_optical_device("sr1", false));

        let drives = hw.optical_drives();
        assert_eq!(drives.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Helper function tests
    // -----------------------------------------------------------------------

    #[test]
    fn is_class_relevant_usb() {
        assert!(is_class_relevant(DeviceClass::UsbStorage));
        assert!(is_class_relevant(DeviceClass::Optical));
        assert!(is_class_relevant(DeviceClass::SdCard));
    }

    #[test]
    fn is_class_relevant_internal_not() {
        assert!(!is_class_relevant(DeviceClass::BlockInternal));
        assert!(!is_class_relevant(DeviceClass::Network));
        assert!(!is_class_relevant(DeviceClass::Loop));
        assert!(!is_class_relevant(DeviceClass::DeviceMapper));
        assert!(!is_class_relevant(DeviceClass::Unknown));
    }

    #[test]
    fn is_media_relevant_usb() {
        let info = make_usb_device("sdb1", true);
        assert!(is_media_relevant(&info));
    }

    #[test]
    fn is_media_relevant_internal_no() {
        let info = DeviceInfo::new(
            DeviceId::new("sda"),
            PathBuf::from("/dev/sda"),
            DeviceClass::BlockInternal,
        );
        assert!(!is_media_relevant(&info));
    }

    #[test]
    fn detect_optical_type_iso9660() {
        let mut info = make_optical_device("sr0", true);
        info.fs_type = Some("iso9660".into());
        let dt = detect_optical_type(&info);
        assert_eq!(dt, yukti::optical::DiscType::CdData);
    }

    #[test]
    fn detect_optical_type_udf() {
        let mut info = make_optical_device("sr0", true);
        info.fs_type = Some("udf".into());
        let dt = detect_optical_type(&info);
        assert_eq!(dt, yukti::optical::DiscType::DvdRom);
    }

    #[test]
    fn detect_optical_type_unknown_fs() {
        let mut info = make_optical_device("sr0", true);
        info.fs_type = Some("ntfs".into());
        let dt = detect_optical_type(&info);
        // Unknown/unrecognized filesystem falls through to yukti's detect_disc_type
        // which returns Unknown for non-media-type strings
        assert_eq!(dt, yukti::optical::DiscType::Unknown);
    }

    #[test]
    fn detect_optical_type_no_fs() {
        let info = make_optical_device("sr0", true);
        let mut no_fs = info;
        no_fs.fs_type = None;
        let dt = detect_optical_type(&no_fs);
        assert_eq!(dt, yukti::optical::DiscType::Unknown);
    }

    #[test]
    fn is_on_removable_device_found() {
        let dev = make_usb_device("sdb1", true);
        let devices = vec![&dev];
        let result = is_on_removable_device(Path::new("/mnt/sdb1/music/song.flac"), &devices);
        assert_eq!(result, Some(DeviceId::new("sdb1")));
    }

    #[test]
    fn is_on_removable_device_not_found() {
        let dev = make_usb_device("sdb1", true);
        let devices = vec![&dev];
        let result = is_on_removable_device(Path::new("/home/user/music/song.flac"), &devices);
        assert!(result.is_none());
    }

    #[test]
    fn is_on_removable_device_empty() {
        let devices: Vec<&DeviceInfo> = vec![];
        let result = is_on_removable_device(Path::new("/mnt/usb/song.flac"), &devices);
        assert!(result.is_none());
    }

    // -----------------------------------------------------------------------
    // Event handling integration tests
    // -----------------------------------------------------------------------

    #[test]
    fn handle_yukti_event_attach() {
        let (mut hw, rx) = HardwareManager::new();
        let info = make_usb_device("sdb1", true);
        let event = DeviceEvent::new(
            DeviceId::new("sdb1"),
            DeviceClass::UsbStorage,
            DeviceEventKind::Attached,
            PathBuf::from("/dev/sdb1"),
        )
        .with_info(info);

        hw.handle_yukti_event(event);

        assert_eq!(hw.devices.len(), 1);
        let hw_event = rx.try_recv().unwrap();
        assert!(matches!(hw_event, HardwareEvent::UsbMounted { .. }));
    }

    #[test]
    fn handle_yukti_event_detach() {
        let (mut hw, rx) = HardwareManager::new();
        let info = make_usb_device("sdb1", true);
        hw.devices.insert(DeviceId::new("sdb1"), info);

        let event = DeviceEvent::new(
            DeviceId::new("sdb1"),
            DeviceClass::UsbStorage,
            DeviceEventKind::Detached,
            PathBuf::from("/dev/sdb1"),
        );

        hw.handle_yukti_event(event);

        assert!(hw.devices.is_empty());
        let hw_event = rx.try_recv().unwrap();
        assert!(matches!(hw_event, HardwareEvent::UsbRemoved { .. }));
    }

    #[test]
    fn handle_yukti_event_mounted() {
        let (mut hw, rx) = HardwareManager::new();
        let info = make_usb_device("sdb1", false);
        hw.devices.insert(DeviceId::new("sdb1"), info);

        let event = DeviceEvent::new(
            DeviceId::new("sdb1"),
            DeviceClass::UsbStorage,
            DeviceEventKind::Mounted {
                mount_point: PathBuf::from("/mnt/usb"),
            },
            PathBuf::from("/dev/sdb1"),
        );

        hw.handle_yukti_event(event);

        let device = hw.devices.get(&DeviceId::new("sdb1")).unwrap();
        assert!(device.is_mounted());
        let hw_event = rx.try_recv().unwrap();
        assert!(matches!(hw_event, HardwareEvent::UsbMounted { .. }));
    }

    #[test]
    fn handle_yukti_event_media_changed_optical() {
        let (mut hw, rx) = HardwareManager::new();
        let info = make_optical_device("sr0", true);
        hw.devices.insert(DeviceId::new("sr0"), info);

        let event = DeviceEvent::new(
            DeviceId::new("sr0"),
            DeviceClass::Optical,
            DeviceEventKind::MediaChanged,
            PathBuf::from("/dev/sr0"),
        );

        hw.handle_yukti_event(event);

        let hw_event = rx.try_recv().unwrap();
        assert!(matches!(hw_event, HardwareEvent::DiscInserted { .. }));
    }

    #[test]
    fn handle_yukti_event_error() {
        let (mut hw, rx) = HardwareManager::new();

        let event = DeviceEvent::new(
            DeviceId::new("sdb1"),
            DeviceClass::UsbStorage,
            DeviceEventKind::Error {
                message: "IO fault".into(),
            },
            PathBuf::from("/dev/sdb1"),
        );

        hw.handle_yukti_event(event);

        let hw_event = rx.try_recv().unwrap();
        match hw_event {
            HardwareEvent::DeviceError { message, .. } => {
                assert_eq!(message, "IO fault");
            }
            _ => panic!("expected DeviceError"),
        }
    }

    #[test]
    fn handle_yukti_event_irrelevant_class_ignored() {
        let (mut hw, rx) = HardwareManager::new();

        let event = DeviceEvent::new(
            DeviceId::new("nvme0n1"),
            DeviceClass::BlockInternal,
            DeviceEventKind::Attached,
            PathBuf::from("/dev/nvme0n1"),
        );

        hw.handle_yukti_event(event);

        assert!(hw.devices.is_empty());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn handle_yukti_event_unmounted() {
        let (mut hw, _rx) = HardwareManager::new();
        let info = make_usb_device("sdb1", true);
        hw.devices.insert(DeviceId::new("sdb1"), info);

        let event = DeviceEvent::new(
            DeviceId::new("sdb1"),
            DeviceClass::UsbStorage,
            DeviceEventKind::Unmounted,
            PathBuf::from("/dev/sdb1"),
        );

        hw.handle_yukti_event(event);

        let device = hw.devices.get(&DeviceId::new("sdb1")).unwrap();
        assert_eq!(device.state, DeviceState::Ready);
        assert!(device.mount_point.is_none());
    }

    #[test]
    fn safe_eject_unknown_device() {
        let (mut hw, _rx) = HardwareManager::new();
        let result = hw.safe_eject(&DeviceId::new("nonexistent"));
        assert!(result.is_err());
    }
}
