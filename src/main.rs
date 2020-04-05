extern crate adi;
extern crate evdev;
extern crate libc;
extern crate twang; 

use libc::{c_int, c_short, c_ulong, c_void};
use std::collections::HashSet;
use std::os::unix::io::AsRawFd;
use std::ptr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{thread};
use adi::speaker::Speaker;
use twang::prelude::SampleSlice;
use twang::Sound;

// Following structs used for udev operations
#[repr(C)]
struct pollfd {
    fd: c_int,
    events: c_short,
    revents: c_short,
}

#[repr(C)]
struct sigset_t {
    __private: c_void,
}

extern "C" {
    fn ppoll(
        fds: *mut pollfd,
        nfds: nfds_t,
        timeout_ts: *mut libc::timespec,
        sigmask: *const sigset_t,
    ) -> c_int;
}

#[allow(non_camel_case_types)]
type nfds_t = c_ulong;

// Used for handling evdev events
fn process(dev: String) {
    let mut d;
    let mut pos: usize = 0;
    let mut devices = evdev::enumerate();

    for (i, d) in devices.iter().enumerate() {
        //println!("{}: {:?}", i, d.name());
        if d.name().clone().into_string().unwrap().contains(&dev) {
            pos = i;
            break;
        }
    }

    d = devices.swap_remove(pos);

    loop {
        for ev in d.events_no_sync().unwrap() {
            if ev._type != 0 && ev.value == 1 {
                println!("{} {} {}", dev, ev._type, ev.value);
            }
        }
    }
}

fn main() {

    // Hold information on what device events have occured
    let dat: HashSet<usize> = HashSet::new();
    let data = Arc::new(Mutex::new(dat));

    let mut speaker = Speaker::new(0, false).unwrap();
    let piano = [
        0.700, 0.243, 0.229, 0.095, 0.139, 0.087, 0.288, 0.199, 0.124, 0.090,
    ];

    let devices = vec!["Headphone", "Lid", "Video Bus"];

    for dev in devices {
        thread::spawn(move || {
            process(dev.to_string());
        });
    }

    let device_data = Arc::clone(&data);

    // Handle USB events
    thread::spawn(move || {

        let mut socket = udev::MonitorBuilder::new()
            .unwrap()
            .match_subsystem_devtype("usb", "usb_device")
            .unwrap()
            .match_subsystem("drm")
            .unwrap()
            .listen()
            .unwrap();

        let mut fds = vec![pollfd {
            fd: socket.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        }];

        loop {
            let _result = unsafe {
                ppoll(
                    (&mut fds[..]).as_mut_ptr(),
                    fds.len() as libc::nfds_t,
                    ptr::null_mut(),
                    ptr::null(),
                )
            };

            let event = match socket.next() {
                Some(evt) => evt,
                None => {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
            };

            if event.subsystem().map_or("", |s| s.to_str().unwrap_or("")) == "usb"
                && (format!("{}", event.event_type()) == "add"
                    || format!("{}", event.event_type()) == "remove")
            {
                let tone = event.sysname().to_str().unwrap_or("");
                let id: Vec<&str> = tone.split(".").collect();
                let mut usb_bus: usize = 0;

                if tone.contains(".") {
                    usb_bus = id[1].parse().unwrap();
                    usb_bus -= 1;
                }

                if format!("{}", event.event_type()) == "add" {
                    let mut d = device_data.lock().unwrap();
                    d.insert(usb_bus as usize);
                } else if format!("{}", event.event_type()) == "remove" {
                    let mut d = device_data.lock().unwrap();
                    d.remove(&(usb_bus as usize));
                }
            }
        }
    });

    // Generate a number of potential tones to select from
    let mut snds = vec![];
    for x in 1..10 {
        snds.push(Sound::new(None, 400.0 + (x as f64 * 100.0)));
    }

    loop {
        speaker.update(&mut || {

            // Obtain sample and advance, for each potential tone
            let mut samps: Vec<twang::Sample> = vec![];
            for v in &mut snds {
                samps.push(v.next().unwrap().har(&piano));
            }

            // Obtain which samples we will play
            let mut play_samps: Vec<twang::Sample> = vec![];

            {
                let device_data = Arc::clone(&data);
                let d = device_data.lock().unwrap();
                for v in &*d {
                    play_samps.push(samps[*v]);
                }
            }

            // Check if we should be playing a tone or not
            if play_samps.len() > 0 {
                play_samps[..].mix().into()
            } else {
                (0 as i16).into()
            }
        });
    }
}
