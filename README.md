## bar

My own bar for [spectrwm](https://github.com/conformal/spectrwm), coded in Rust (and a little bit of C).

![bar](https://image.petitmur.beer/bar.png)

### features

* date and time
* battery (level, status, design level based)
* wireless (state, essid, signal strength, speed)
* audio sink and source (level, muted)
* brightness
* cpu usage and temperature
* memory (percent or used/total in gigaoctet)
* dynamic icons, colors
* configuration in YAML

All the info is read direclty from the kernel files (`/sys`, `/proc`). Except for the audio and wireless modules.\
The audio module communicates with the [PulseAudio](https://www.freedesktop.org/wiki/Software/PulseAudio/) server through the [client API](https://freedesktop.org/software/pulseaudio/doxygen/) to retrieve its data.\
Wireless module use the [802.11 netlink interface](https://www.infradead.org/~tgr/libnl/) to retrieve its data.

Some modules run in their own thread.

### prerequisite

- libnl (for the wireless module)
- pulseaudio (for sound and mic module)
- an icon font installed (I use [this](https://github.com/Templarian/MaterialDesign-Font))

### configuration

The binary looks for the config file `bar.yaml` located in `$HOME/.config/bar/`.\
If the config file is not found, the bar prints an error and exits.\
All options are detailed [here](https://github.com/doums/bar/blob/master/bar.yaml).

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

### license
Mozilla Public License 2.0
