# Usb Midi stompbox in rust

This project is an embedded firmware to create a 'stompbox' that
sends midi signals over usb.

This is similar in concept to
https://www.behringer.com/Categories/Behringer/Accessories/Midi-Foot-Controllers/FCB1010/p/P0089#googtrans(en|en)

# Design

This is currently using RTFM to provide task scheduling.
The usb drivers are implemented using the usb-hal.
Ultimately the bulk of the logic is in the usbd-midi crate.

Hopefully this should be compatible with other devices that support the HAL.

While there is existing products and even firmwares that achieve this, this
project aims to use embedded rust to explore how useful rust is in an
embedded context.

# Priorities

This currently uses RTFM's priorities to schedule 'tasks' as follows.
1. Processing usb
2. Sending Midi
3. Reading IO/Status LEd

Processing usb needs to be done at the highest priority, otherwise the device
can 'malfunction' in the usb stack. This is also interrupt driven,
so we are handling the USB side fast.

Then the sending of MIDI messages into the usb stack is prioritized.

The actually IO is currently ran with the lowest priority, as if things
are missed here it is not the end of the world.


# What works

Currently will send a midi message when the state of PA4 (bluepill)
changes. Will send a MIDI ON message on the rising edge, and a MIDI OFF message
on the falling edge.

The whole system is almost entirely interrupt driven, only the LED blinking is
not, but this may be possible using PWM instead, and changing the LED based on
panics/other tasks.

# Contributions

This is my first endevaour in rust and embedded rust, feedback is welcome.
Modifying the build process to support multiple boards would 
be an awesome contribution.

