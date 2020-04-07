# synth

A synth controlled by unplugging and plugging devices into your computer (e.g. plugging in usb drive) and your laptop lid.  

Different usb ports, give different tones!

This makes use of alsa, so first stop pulseaudio if necessary:

```
systemctl --user stop pulseaudio.socket
systemctl --user stop pulseaudio.service
```

Run the synth (needs root to monitor events):

```
cargo build
sudo ./target/debug/synth
```

Re-enable pulseaudio when finished:

```
systemctl --user start pulseaudio.socket
systemctl --user start pulseaudio.service
```
