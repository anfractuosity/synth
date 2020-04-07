extern crate adi;
extern crate evdev;
extern crate libc;
extern crate twang;
extern crate serde_derive; 
extern crate toml;

use std::collections::HashMap;
use serde_derive::Deserialize;
use libc::{c_int, c_short, c_ulong, c_void};
use std::collections::HashSet;
use std::os::unix::io::AsRawFd;
use std::ptr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{thread};
use std::fs;
use std::fs::{File, OpenOptions};
use adi::speaker::Speaker;
use twang::prelude::SampleSlice;
use twang::Sound;

#[derive(Deserialize,Clone)]
struct Usb {
    port: String,
    tone: f64
}

#[derive(Deserialize,Clone)]
struct Lid {
    tone: f64
}

#[derive(Deserialize,Clone)]
struct Config {
    usb: Vec<Usb>,
    lid: Option<Lid>,
}

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
fn process(dev: String, config : Config, device_data : Arc<Mutex<HashSet<String>>>) {
    let mut d;
    let mut pos: usize = 0;
    let mut devices = evdev::enumerate();
    let mut found = false;

    for (i, d) in devices.iter().enumerate() {
        //println!("{}: {:?}", i, d.name());
        if d.name().clone().into_string().unwrap().contains(&dev) {
            pos = i;
            found = true;
            break;
        }
    }
    
    if !found {
        return;
    }
    
    d = devices.swap_remove(pos);

    loop {
        for ev in d.events_no_sync().unwrap() {
            if ev._type != 0 {
                
                let mut tone = -1.0;
                if dev == "Lid" {
                    tone = config.clone().lid.unwrap().tone ;
                }

                if tone > 0.0 {
                    if ev.value != 0 {
                        let mut d = device_data.lock().unwrap();
                        d.insert(format!("{}#{}",dev,tone));
                    } else {
                        let mut d = device_data.lock().unwrap();
                        d.remove(&(format!("{}#{}",dev,tone)));
                    }
                }
            }
        }
    }
}

fn main() {

    let config_file = "synth.toml";

    let contents = fs::read_to_string(config_file)
        .expect("Config file not found");

    let config: Config = toml::from_str(&contents).unwrap();

    // Hold information on what device events have occured
    let mut dat: HashSet<String> = HashSet::new();
    let data = Arc::new(Mutex::new(dat));

    let mut speaker = Speaker::new(0, false).unwrap();
    let piano = [
        0.700, 0.243, 0.229, 0.095, 0.139, 0.087, 0.288, 0.199, 0.124, 0.090,
    ];

    let devices = vec!["Headphone", "Lid", "Video Bus"];

    for dev in devices {
        let cfg = config.clone();
        let data = Arc::clone(&data);
        thread::spawn(move || {
            process(dev.to_string(),cfg,data);
        });
    }

    let device_data = Arc::clone(&data);
    let config_tmp = config.clone();

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

            let etype = format!("{}", event.event_type()) ;

            if event.subsystem().map_or("", |s| s.to_str().unwrap_or("")) == "usb"
                && (etype == "add" || etype == "remove"){
                let usbport = event.sysname().to_str().unwrap_or("");

                println!("USB info: {} {}",etype,usbport);

                for usb in &config_tmp.usb {
                    if usb.port == usbport {                   
                        if etype == "add" {
                            let mut d = device_data.lock().unwrap();
                            d.insert(format!("{}#{}",usb.port,usb.tone));
                        } else if etype == "remove" {
                            let mut d = device_data.lock().unwrap();
                            d.remove(&(format!("{}#{}",usb.port,usb.tone)));
                        }
 
                    }
                }
            }
        }
    });

    // Generate a number of potential tones to select from
    let mut snds = HashMap::new();
    for x in &config.usb {
        snds.insert(format!("{}#{}",x.port,x.tone),Sound::new(None,x.tone));
    }

    let ltone = config.lid.unwrap().tone;
    snds.insert(format!("Lid#{}",ltone),Sound::new(None,ltone));
    
    loop {
        speaker.update(&mut || {

            // Obtain sample and advance, for each potential tone
            //let mut samps: Vec<twang::Sample> = vec![];
            let mut play_samps: Vec<twang::Sample> = vec![];
            {
                let device_data = Arc::clone(&data);
                let d = device_data.lock().unwrap();
                for v in &*d {
                    for (key,value) in &mut snds {
                        let val = value.next().unwrap().har(&piano);
                        if key == v {
                            play_samps.push(val);
                        } 
                    }
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
