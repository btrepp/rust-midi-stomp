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
embedded context

# Contributions

This is my first endevaour in rust and embedded rust, feedback is welcome.
Modifying the build process to support multiple boards would 
be an awesome contribution.

