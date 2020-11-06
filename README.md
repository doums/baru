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

All the info is read direclty from the kernel files (`/sys`, `/proc`). Except audio and network modules which use libraries.\
All modules are threaded.
Thanks to this design (as well Rust and C), baru is lightweight and efficient. It can run at a refresh rate of 60 "fps" with a minimal processor footprint.

The audio module communicates with the [PulseAudio](https://www.freedesktop.org/wiki/Software/PulseAudio/) server through the [client API](https://freedesktop.org/software/pulseaudio/doxygen/) to retrieve its data.\
Wireless and wired modules use the netlink interface with the help of [libnl](https://www.infradead.org/~tgr/libnl/) to talk directly to kernel and retrieve their data.\
In addition, wireless module uses the [802.11](https://github.com/torvalds/linux/blob/master/include/uapi/linux/nl80211.h) API.

Baru is modular. This means that only the modules you want to see are instantiated and executed.

### prerequisite

- libnl (for wired and wireless modules)
- pulseaudio (for sound and mic modules)

### configuration

The binary looks for the config file `baru.yaml` located in `$XDG_CONFIG_HOME/baru/` (default to `$HOME/.config/baru/`).\
If the config file is not found, baru prints an error and exits.\
All options are detailed [here](https://github.com/doums/baru/blob/master/baru.yaml).

Example:
```yaml
bar: '%c  %t  %b  %s   %w %a   %d'
tick: 50
memory:
  display: GiB
mic:
  index: 1
temperature:
  core_inputs: 2..5
wired:
  discrete: true
wireless:
  interface: wlp2s0
  display: Essid
  max_essid_len: 4
```

### usage
```
$ baru
```

### credits

Clément Dommerc for providing me with the C code for the lib `nl_data`, wireless part.

### license
Mozilla Public License 2.0
