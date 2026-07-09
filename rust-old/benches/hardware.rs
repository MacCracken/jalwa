//! Benchmarks for the hardware device management module.

#![cfg(feature = "yukti")]

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use jalwa_core::hardware::{HardwareManager, is_on_removable_device};
use std::path::PathBuf;
use yukti::device::{DeviceClass, DeviceId, DeviceInfo, DeviceState};
use yukti::event::{DeviceEvent, DeviceEventKind};

// ---------------------------------------------------------------------------
// Helpers (mirroring the test helpers in hardware.rs)
// ---------------------------------------------------------------------------

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
    info.label = Some(format!("USB Drive {id}"));
    info.size_bytes = 32_000_000_000; // 32 GB
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

/// Generate a USB attach/mount/detach event cycle for a single device.
fn make_usb_cycle(idx: usize) -> Vec<DeviceEvent> {
    let id_str = format!("sdb{idx}");
    let dev_path = PathBuf::from(format!("/dev/{id_str}"));
    let device_id = DeviceId::new(&id_str);
    let info = make_usb_device(&id_str, false);

    vec![
        DeviceEvent::new(
            device_id.clone(),
            DeviceClass::UsbStorage,
            DeviceEventKind::Attached,
            dev_path.clone(),
        )
        .with_info(info),
        DeviceEvent::new(
            device_id.clone(),
            DeviceClass::UsbStorage,
            DeviceEventKind::Mounted {
                mount_point: PathBuf::from(format!("/mnt/{id_str}")),
            },
            dev_path.clone(),
        ),
        DeviceEvent::new(
            device_id,
            DeviceClass::UsbStorage,
            DeviceEventKind::Detached,
            dev_path,
        ),
    ]
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_hardware_event_processing(c: &mut Criterion) {
    // Pre-generate 100 device events (attach/mount/detach cycles).
    // 100 / 3 = ~33 full cycles plus one extra attach.
    let events: Vec<DeviceEvent> = (0..34).flat_map(make_usb_cycle).take(100).collect();

    c.bench_function("hardware_event_processing_100", |b| {
        b.iter_with_setup(
            || {
                let (hw, _rx) = HardwareManager::new();
                (hw, events.clone())
            },
            |(mut hw, evts): (HardwareManager, Vec<DeviceEvent>)| {
                for event in evts {
                    hw.handle_yukti_event(black_box(event));
                }
            },
        );
    });
}

fn bench_device_lookup(c: &mut Criterion) {
    c.bench_function("mounted_usb_devices_20", |b| {
        b.iter_with_setup(
            || {
                let (mut hw, _rx) = HardwareManager::new();
                // Insert 20 devices: 10 mounted USB, 5 unmounted USB, 5 optical.
                for i in 0..10 {
                    let info = make_usb_device(&format!("usb_m{i}"), true);
                    hw.handle_yukti_event(
                        DeviceEvent::new(
                            info.id.clone(),
                            DeviceClass::UsbStorage,
                            DeviceEventKind::Attached,
                            info.dev_path.clone(),
                        )
                        .with_info(info),
                    );
                }
                for i in 0..5 {
                    let info = make_usb_device(&format!("usb_u{i}"), false);
                    hw.handle_yukti_event(
                        DeviceEvent::new(
                            info.id.clone(),
                            DeviceClass::UsbStorage,
                            DeviceEventKind::Attached,
                            info.dev_path.clone(),
                        )
                        .with_info(info),
                    );
                }
                for i in 0..5 {
                    let info = make_optical_device(&format!("sr{i}"), i % 2 == 0);
                    hw.handle_yukti_event(
                        DeviceEvent::new(
                            info.id.clone(),
                            DeviceClass::Optical,
                            DeviceEventKind::Attached,
                            info.dev_path.clone(),
                        )
                        .with_info(info),
                    );
                }
                (hw, _rx)
            },
            |(hw, _rx)| {
                let _mounted = black_box(hw.mounted_usb_devices());
                let _optical = black_box(hw.optical_drives());
            },
        );
    });
}

fn bench_is_on_removable_device(c: &mut Criterion) {
    // Create 10 mounted USB devices.
    let devices: Vec<DeviceInfo> = (0..10)
        .map(|i| make_usb_device(&format!("usb{i}"), true))
        .collect();
    let device_refs: Vec<&DeviceInfo> = devices.iter().collect();

    let hit_path = PathBuf::from("/mnt/usb5/music/album/song.flac");
    let miss_path = PathBuf::from("/home/user/music/local.flac");

    c.bench_function("is_on_removable_device_hit", |b| {
        b.iter(|| {
            black_box(is_on_removable_device(
                black_box(&hit_path),
                black_box(&device_refs),
            ));
        });
    });

    c.bench_function("is_on_removable_device_miss", |b| {
        b.iter(|| {
            black_box(is_on_removable_device(
                black_box(&miss_path),
                black_box(&device_refs),
            ));
        });
    });
}

fn bench_device_info_display(c: &mut Criterion) {
    // Devices with different states: label set, model only, neither; various sizes.
    let mut with_label = make_usb_device("sdb1", true);
    with_label.label = Some("My USB Drive".into());
    with_label.size_bytes = 64_000_000_000;

    let mut with_model = DeviceInfo::new(
        DeviceId::new("sdc1"),
        PathBuf::from("/dev/sdc1"),
        DeviceClass::UsbStorage,
    );
    with_model.model = Some("SanDisk Cruzer".into());
    with_model.size_bytes = 16_000_000_000;

    let bare = DeviceInfo::new(
        DeviceId::new("sdd1"),
        PathBuf::from("/dev/sdd1"),
        DeviceClass::UsbStorage,
    );

    let devices = [with_label, with_model, bare];

    c.bench_function("device_info_display_name", |b| {
        b.iter(|| {
            for dev in devices.iter() {
                let name = dev.display_name();
                black_box(&name);
            }
        });
    });

    c.bench_function("device_info_size_display", |b| {
        b.iter(|| {
            for dev in devices.iter() {
                let size = dev.size_display();
                black_box(&size);
            }
        });
    });
}

criterion_group!(
    benches,
    bench_hardware_event_processing,
    bench_device_lookup,
    bench_is_on_removable_device,
    bench_device_info_display,
);
criterion_main!(benches);
