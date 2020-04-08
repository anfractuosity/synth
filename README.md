# synth

A synth controlled by unplugging and plugging devices into your computer (e.g. plugging in usb drive) and your laptop lid.  

Different usb ports, give different tones!

Click below to see the amazing(ly terrible) video: 

[![synth video](/images/synth.png)](https://www.youtube.com/watch?v=rY7_q5ujVkw)

This makes use of alsa, so first stop pulseaudio if necessary:

```
systemctl --user stop pulseaudio.socket
systemctl --user stop pulseaudio.service
```

Run the synth (needs root to monitor events):

```
cargo build
sudo -E cargo run
```

Re-enable pulseaudio when finished:

```
systemctl --user start pulseaudio.socket
systemctl --user start pulseaudio.service
```

Using the laptop lid for controlling something was inspired by this excellent repo for generating morse code with your laptop - https://github.com/veggiedefender/open-and-shut

# To Do

* Better error handling - e.g. if fields in the .toml config file aren't present
* Stop audio loop using 100% of a CPU core
