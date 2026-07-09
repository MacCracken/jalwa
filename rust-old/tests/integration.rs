//! Integration tests — cross-module scenarios for jalwa.

#![allow(unused_imports)]

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Hardware integration tests (require yukti feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "yukti")]
mod hardware {
    use jalwa_core::hardware::{HardwareEvent, HardwareManager, is_on_removable_device};
    use jalwa_core::test_fixtures::make_media_item;
    use std::path::PathBuf;
    use yukti::device::{DeviceClass, DeviceId, DeviceInfo, DeviceState};
    use yukti::event::{DeviceEvent, DeviceEventKind};

    // -- helpers --

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

    /// Full USB lifecycle: attach (mounted) -> verify UsbMounted -> detach -> verify UsbRemoved.
    #[test]
    fn hardware_usb_lifecycle() {
        let (mut hw, rx) = HardwareManager::new();

        // Step 1: Attach a mounted USB device.
        let usb_info = make_usb_device("sdb1", true);
        let attach_event = DeviceEvent::new(
            DeviceId::new("sdb1"),
            DeviceClass::UsbStorage,
            DeviceEventKind::Attached,
            PathBuf::from("/dev/sdb1"),
        )
        .with_info(usb_info);

        hw.handle_yukti_event(attach_event);

        // Verify the device is tracked.
        assert_eq!(hw.devices().len(), 1);
        assert_eq!(hw.mounted_usb_devices().len(), 1);

        // Verify UsbMounted event was emitted.
        let event = rx.try_recv().expect("expected UsbMounted event");
        match event {
            HardwareEvent::UsbMounted {
                device_id,
                mount_point,
                ..
            } => {
                assert_eq!(device_id, DeviceId::new("sdb1"));
                assert_eq!(mount_point, PathBuf::from("/mnt/sdb1"));
            }
            other => panic!("expected UsbMounted, got {other:?}"),
        }

        // Step 2: Unmount the device.
        let unmount_event = DeviceEvent::new(
            DeviceId::new("sdb1"),
            DeviceClass::UsbStorage,
            DeviceEventKind::Unmounted,
            PathBuf::from("/dev/sdb1"),
        );
        hw.handle_yukti_event(unmount_event);

        // Device is still tracked but no longer mounted.
        assert_eq!(hw.devices().len(), 1);
        assert_eq!(hw.mounted_usb_devices().len(), 0);

        // Step 3: Detach the device.
        let detach_event = DeviceEvent::new(
            DeviceId::new("sdb1"),
            DeviceClass::UsbStorage,
            DeviceEventKind::Detached,
            PathBuf::from("/dev/sdb1"),
        );
        hw.handle_yukti_event(detach_event);

        // Device should be removed from tracking.
        assert!(hw.devices().is_empty());

        // Verify UsbRemoved event.
        let event = rx.try_recv().expect("expected UsbRemoved event");
        assert!(matches!(event, HardwareEvent::UsbRemoved { .. }));
    }

    /// Full optical lifecycle: insert disc -> media change -> eject (detach).
    #[test]
    fn hardware_optical_lifecycle() {
        let (mut hw, rx) = HardwareManager::new();

        // Step 1: Attach an optical drive with media.
        let optical_info = make_optical_device("sr0", true);
        let attach_event = DeviceEvent::new(
            DeviceId::new("sr0"),
            DeviceClass::Optical,
            DeviceEventKind::Attached,
            PathBuf::from("/dev/sr0"),
        )
        .with_info(optical_info);

        hw.handle_yukti_event(attach_event);

        assert_eq!(hw.optical_drives().len(), 1);

        // DiscInserted should have been emitted (device has media).
        let event = rx.try_recv().expect("expected DiscInserted on attach");
        match &event {
            HardwareEvent::DiscInserted { device_id, .. } => {
                assert_eq!(*device_id, DeviceId::new("sr0"));
            }
            other => panic!("expected DiscInserted, got {other:?}"),
        }

        // Step 2: Media change (e.g. user swaps disc).
        let media_event = DeviceEvent::new(
            DeviceId::new("sr0"),
            DeviceClass::Optical,
            DeviceEventKind::MediaChanged,
            PathBuf::from("/dev/sr0"),
        );
        hw.handle_yukti_event(media_event);

        let event = rx
            .try_recv()
            .expect("expected DiscInserted on media change");
        assert!(matches!(event, HardwareEvent::DiscInserted { .. }));

        // Step 3: Eject (detach).
        let detach_event = DeviceEvent::new(
            DeviceId::new("sr0"),
            DeviceClass::Optical,
            DeviceEventKind::Detached,
            PathBuf::from("/dev/sr0"),
        );
        hw.handle_yukti_event(detach_event);

        assert!(hw.devices().is_empty());

        let event = rx.try_recv().expect("expected DiscEjected");
        assert!(matches!(event, HardwareEvent::DiscEjected { .. }));
    }

    /// Register a playback path on a USB device, then simulate device removal.
    /// Verify PlaybackDeviceRemoved is emitted before UsbRemoved.
    #[test]
    fn hardware_playback_device_removal() {
        let (mut hw, rx) = HardwareManager::new();

        // Pre-populate device (simulates a previously attached USB).
        let usb_info = make_usb_device("sdc1", true);
        let id = usb_info.id.clone();
        hw.handle_yukti_event(
            DeviceEvent::new(
                id.clone(),
                DeviceClass::UsbStorage,
                DeviceEventKind::Attached,
                PathBuf::from("/dev/sdc1"),
            )
            .with_info(usb_info),
        );

        // Drain the UsbMounted event from attach.
        let _ = rx.try_recv();

        // Register a playback path on this device.
        hw.register_playback_path(PathBuf::from("/mnt/sdc1/music/album/track03.flac"));

        // Now detach the device.
        let detach = DeviceEvent::new(
            DeviceId::new("sdc1"),
            DeviceClass::UsbStorage,
            DeviceEventKind::Detached,
            PathBuf::from("/dev/sdc1"),
        );
        hw.handle_yukti_event(detach);

        // First event: PlaybackDeviceRemoved (because we were playing from this mount).
        let event1 = rx.try_recv().expect("expected PlaybackDeviceRemoved");
        match event1 {
            HardwareEvent::PlaybackDeviceRemoved {
                device_id,
                mount_point,
            } => {
                assert_eq!(device_id, DeviceId::new("sdc1"));
                assert_eq!(mount_point, PathBuf::from("/mnt/sdc1"));
            }
            other => panic!("expected PlaybackDeviceRemoved, got {other:?}"),
        }

        // Second event: UsbRemoved.
        let event2 = rx.try_recv().expect("expected UsbRemoved");
        assert!(matches!(event2, HardwareEvent::UsbRemoved { .. }));
    }

    /// Verify that when a USB device is mounted, the mount point can be used
    /// as a library scan path.
    #[test]
    fn hardware_usb_auto_scan_path() {
        let (mut hw, rx) = HardwareManager::new();

        // Attach an unmounted USB first.
        let usb_info = make_usb_device("sdd1", false);
        hw.handle_yukti_event(
            DeviceEvent::new(
                DeviceId::new("sdd1"),
                DeviceClass::UsbStorage,
                DeviceEventKind::Attached,
                PathBuf::from("/dev/sdd1"),
            )
            .with_info(usb_info),
        );

        // No UsbMounted yet (not mounted).
        assert!(rx.try_recv().is_err());

        // Mount event arrives.
        let mount_event = DeviceEvent::new(
            DeviceId::new("sdd1"),
            DeviceClass::UsbStorage,
            DeviceEventKind::Mounted {
                mount_point: PathBuf::from("/media/user/MyMusic"),
            },
            PathBuf::from("/dev/sdd1"),
        );
        hw.handle_yukti_event(mount_event);

        // Now we should receive UsbMounted with the mount point.
        let event = rx.try_recv().expect("expected UsbMounted after mount");
        let mount_point = match event {
            HardwareEvent::UsbMounted { mount_point, .. } => mount_point,
            other => panic!("expected UsbMounted, got {other:?}"),
        };

        // The mount point should be usable as a scan path for the library.
        let mut lib = jalwa_core::Library::new();
        lib.add_scan_path(mount_point.clone());
        assert!(lib.scan_paths.contains(&mount_point));
        assert_eq!(mount_point, PathBuf::from("/media/user/MyMusic"));
    }

    /// Create library items and check is_on_removable_device against device info.
    #[test]
    fn library_with_hardware_devices() {
        // Set up devices.
        let usb_dev = make_usb_device("sdb1", true); // mounted at /mnt/sdb1
        let optical_dev = make_optical_device("sr0", true); // no mount point (optical)

        let devices: Vec<&DeviceInfo> = vec![&usb_dev, &optical_dev];

        // Create a media item on the USB device.
        let mut item_on_usb = make_media_item("USB Song", "USB Artist", 180);
        item_on_usb.path = PathBuf::from("/mnt/sdb1/music/song.flac");

        // Create a media item on the internal drive.
        let mut item_internal = make_media_item("Local Song", "Local Artist", 240);
        item_internal.path = PathBuf::from("/home/user/Music/song.flac");

        // The USB item should be detected as on a removable device.
        let result = is_on_removable_device(&item_on_usb.path, &devices);
        assert_eq!(result, Some(DeviceId::new("sdb1")));

        // The internal item should NOT be detected as removable.
        let result = is_on_removable_device(&item_internal.path, &devices);
        assert!(result.is_none());

        // Verify the library can hold both items and search works across them.
        let mut lib = jalwa_core::Library::new();
        let usb_id = lib.add_item(item_on_usb);
        let local_id = lib.add_item(item_internal);

        assert!(lib.find_by_id(usb_id).is_some());
        assert!(lib.find_by_id(local_id).is_some());

        // Search should find items regardless of device.
        assert_eq!(lib.search("USB Song").len(), 1);
        assert_eq!(lib.search("Local Song").len(), 1);
    }
}

// ---------------------------------------------------------------------------
// Non-yukti integration tests (always run)
// ---------------------------------------------------------------------------

/// Open a database, add items, reopen the database, verify items persist.
#[test]
fn library_persistent_roundtrip() {
    use jalwa_core::db::PersistentLibrary;
    use jalwa_core::test_fixtures::make_media_item;

    let db_path =
        std::env::temp_dir().join(format!("jalwa_integ_roundtrip_{}.db", uuid::Uuid::new_v4()));

    // Phase 1: create library, add items.
    {
        let mut plib = PersistentLibrary::open(&db_path).unwrap();

        let item1 = make_media_item("Roundtrip Song A", "Artist A", 200);
        let item2 = make_media_item("Roundtrip Song B", "Artist B", 300);
        let id1 = plib.add_item(item1).unwrap();
        let id2 = plib.add_item(item2).unwrap();

        // Verify in-memory state.
        assert_eq!(plib.library.items.len(), 2);
        assert!(plib.library.find_by_id(id1).is_some());
        assert!(plib.library.find_by_id(id2).is_some());
    }
    // PersistentLibrary is dropped here, closing the DB connection.

    // Phase 2: reopen and verify items survived.
    {
        let plib = PersistentLibrary::open(&db_path).unwrap();
        assert_eq!(plib.library.items.len(), 2);

        let titles: Vec<&str> = plib
            .library
            .items
            .iter()
            .map(|i| i.title.as_str())
            .collect();
        assert!(titles.contains(&"Roundtrip Song A"));
        assert!(titles.contains(&"Roundtrip Song B"));

        // Verify search works on reloaded data.
        assert_eq!(plib.library.search("Roundtrip").len(), 2);
        assert_eq!(plib.library.search("Artist A").len(), 1);
    }

    let _ = std::fs::remove_file(&db_path);
}

/// Create a playlist, export to M3U, reimport, and verify paths match.
#[test]
fn playlist_io_roundtrip() {
    use jalwa_core::playlist_io::{load_m3u, save_m3u};
    use jalwa_core::test_fixtures::make_media_item;
    use jalwa_core::{Library, Playlist};

    let mut lib = Library::new();

    let mut item_a = make_media_item("Song Alpha", "Artist X", 210);
    item_a.path = PathBuf::from("/music/alpha.flac");
    let mut item_b = make_media_item("Song Beta", "Artist Y", 185);
    item_b.path = PathBuf::from("/music/beta.flac");
    let mut item_c = make_media_item("Song Gamma", "Artist Z", 300);
    item_c.path = PathBuf::from("/music/gamma.flac");

    let id_a = lib.add_item(item_a);
    let id_b = lib.add_item(item_b);
    let id_c = lib.add_item(item_c);

    // Build a playlist with all three items.
    let mut playlist = Playlist::new("Integration Test Playlist");
    playlist.add(id_a);
    playlist.add(id_b);
    playlist.add(id_c);

    assert_eq!(playlist.len(), 3);

    // Export to M3U.
    let m3u_path =
        std::env::temp_dir().join(format!("jalwa_integ_playlist_{}.m3u", uuid::Uuid::new_v4()));
    save_m3u(&playlist, &lib, &m3u_path).unwrap();

    // Re-import and verify paths match.
    let loaded_paths = load_m3u(&m3u_path).unwrap();
    assert_eq!(loaded_paths.len(), 3);
    assert_eq!(loaded_paths[0], PathBuf::from("/music/alpha.flac"));
    assert_eq!(loaded_paths[1], PathBuf::from("/music/beta.flac"));
    assert_eq!(loaded_paths[2], PathBuf::from("/music/gamma.flac"));

    // Verify we can match loaded paths back to library items.
    for path in &loaded_paths {
        let found = lib.find_by_path(path);
        assert!(found.is_some(), "path {path:?} should exist in library");
    }

    let _ = std::fs::remove_file(&m3u_path);
}
