use usb_device::bus::UsbBus;
use usb_device::device::UsbVidPid;
use usb_device::bus::UsbBusAllocator;
use usb_device::prelude::UsbDeviceBuilder;
use usbd_midi::midi_device::MidiClass;
use usbd_midi::data::usb::constants::USB_CLASS_NONE;
use usb_device::device::UsbDevice;

/// Configures the usb device as seen by the operating system.
pub fn configure_usb<'a,B: UsbBus>
                        (usb_bus: &'a UsbBusAllocator<B>) 
                                        -> UsbDevice<'a,B> {
    let usb_vid_pid = UsbVidPid(0x16c0, 0x27dd);
    let usb_dev =
        UsbDeviceBuilder::new(usb_bus,usb_vid_pid )
            .manufacturer("btrepp")
            .product("Rust Midi Stomp")
            .serial_number("1")
            .device_class(USB_CLASS_NONE)
            .build();   
    usb_dev
}

/// Called to process any usb events
/// Note: this needs to be called often,
/// and seemingly always the same way
pub fn usb_poll<B: UsbBus>(
    usb_dev: &mut UsbDevice<'static, B>,
    midi: &mut MidiClass<'static, B>,
) {
    if !usb_dev.poll(&mut [midi]) {
        return;
    }   
}
