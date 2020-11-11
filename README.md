## baru

A system monitor written in Rust and C.

![baru](https://raw.githubusercontent.com/doums/baru/master/public/baru.png)

### features

* date and time
* battery (level, status, design level based)
* wireless (state, essid, signal strength)
* wired (state)
* audio sink and source (level, muted)
* brightness
* cpu usage and temperature
* memory (percent or used/total in gigabyte/gibibyte)
* dynamic and customizable labels
* customizable format output
* configuration in YAML

All the info is read directly from the kernel files (`/sys`, `/proc`). Except audio and network modules which use C libraries.\
There is no memory leak over time. All modules are threaded. Thanks to this design (as well Rust and C), baru is lightweight and efficient. It can run at high refresh rate with a minimal processor footprint.

The audio module communicates with the [PulseAudio](https://www.freedesktop.org/wiki/Software/PulseAudio/) server through the [client API](https://freedesktop.org/software/pulseaudio/doxygen/) to retrieve its data.\
Wireless and wired modules use the netlink interface with the help of [libnl](https://www.infradead.org/~tgr/libnl/) to talk directly to kernel and retrieve their data.\
In addition, wireless module uses the [802.11](https://github.com/torvalds/linux/blob/master/include/uapi/linux/nl80211.h) API.

Baru is modular. This means that only the modules you want to see are instantiated and executed.

### prerequisite

- libnl (for wired and wireless modules)
- libpulse (for sound and mic modules)

### configuration

The binary looks for the config file `baru.yaml` located in `$XDG_CONFIG_HOME/baru/` (default to `$HOME/.config/baru/`).\
If the config file is not found, baru prints an error and exits.\
All options are detailed [here](https://github.com/doums/baru/blob/master/baru.yaml).

Example:
```yaml
format: '%m  %c  %t  %b  %i  %s   %w%e  %a    %d'
tick: 50
battery:
  full_design: true
  low_level: 30
  full_label: '*'
  charging_label: '^'
  discharging_label: 'b'
  low_label: '!'
  unknown_label: '?'
  format: '%l %v'
brightness:
  label: 'l'
  format: '%l %v'
cpu:
  label: 'c'
  high_label: '!'
  format: '%v %l'
memory:
  core_inputs: 2..5
  label: 'm'
  high_label: '!'
  format: '%v %l'
mic:
  index: 1
  label: 'i'
  mute_label: '.'
  format: '%v %l'
sound:
  index: 0
  label: 's'
  mute_label: '.'
  format: '%v %l'
temperature:
  core_inputs: 2..5
  label: 't'
  high_label: '!'
  format: '%v %l'
wired:
  discrete: true
  label: 'e'
  disconnected_label: '\'
  format: '%l'
wireless:
  interface: wlan0
  display: Essid
  max_essid_len: 5
  label: 'w'
  disconnected_label: '\'
  format: '%v %l'
```

### usage
```
$ baru
```

### credits

Clément Dommerc for providing me with the C code for the lib `netlink`, wireless part.

### license
Mozilla Public License 2.0
