## baru

My own bar for [spectrwm](https://github.com/conformal/spectrwm), coded in Rust (and a little bit of C).

![baru](https://image.petitmur.beer/bar.png)

### features

* date and time
* battery (level, status, design level based)
* wireless (state, essid, signal strength)
* wired (state)
* audio sink and source (level, muted)
* brightness
* cpu usage and temperature
* memory (percent or used/total in gigaoctet)
* dynamic icons, colors
* configuration in YAML

All the info is read direclty from the kernel files (`/sys`, `/proc`). Except for the audio and wireless modules.\
The audio module communicates with the [PulseAudio](https://www.freedesktop.org/wiki/Software/PulseAudio/) server through the [client API](https://freedesktop.org/software/pulseaudio/doxygen/) to retrieve its data.\
Wireless and wired modules use the netlink interface with the help of [libnl](https://www.infradead.org/~tgr/libnl/) to talk directly to kernel and retrieve their data.\
In addition, wireless module uses the [802.11](https://github.com/torvalds/linux/blob/master/include/uapi/linux/nl80211.h) API.

Some modules run in their own thread.

### prerequisite

- libnl (for wireless module)
- pulseaudio (for sound and mic modules)
- an icon font installed (I use [this](https://github.com/Templarian/MaterialDesign-Font))

### configuration

The binary looks for the config file `baru.yaml` located in `$HOME/.config/baru/`.\
If the config file is not found, the bar prints an error and exits.\
All options are detailed [here](https://github.com/doums/baru/blob/master/baru.yaml).

Example:
```yaml
bar: '%c  %t  %b  %s   %w %a   %d'
tick: 50
default_font: +@fn=0;
icon_font: +@fn=1;
default_color: +@fg=0;
red: +@fg=1;
green: +@fg=2;
mic:
  index: 1
wireless:
  interface: wlp2s0
  display: Essid
  max_essid_len: 4
temperature:
  core_inputs: 2..5
```

### credits

Clément Dommerc for providing me with the C code for the lib `nl_data`, wireless part.

### license
Mozilla Public License 2.0
