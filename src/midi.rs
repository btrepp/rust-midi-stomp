use usb_device::class_prelude::*;
use usb_device::Result;

pub const USB_CLASS_NONE : u8 = 0x00;
const USB_AUDIO_CLASS: u8 = 0x01;
const USB_AUDIOCONTROL_SUBCLASS: u8 = 0x01;
const USB_MIDISTREAMING_SUBCLASS: u8 =0x03;
const MIDI_IN_JACK_SUBTYPE : u8 = 0x02;
const MIDI_OUT_JACK_SUBTYPE : u8 = 0x03;
const EMBEDDED : u8 = 0x01;
const CS_INTERFACE: u8 = 0x24;
const CS_ENDPOINT: u8 = 0x25;
const HEADER_SUBTYPE: u8 = 0x01;
const MS_HEADER_SUBTYPE: u8 = 0x01;
const MS_GENERAL: u8 = 0x01;

pub struct MidiClass<'a,B: UsbBus> {
    standard_ac: InterfaceNumber,
    standard_mc: InterfaceNumber,
    standard_bulkout: EndpointOut<'a, B>,
    standard_bulkin: EndpointIn<'a,B>
}


impl<B: UsbBus> MidiClass<'_, B> {
    /// Creates a new MidiClass with the provided UsbBus
    pub fn new(alloc: &UsbBusAllocator<B>) -> MidiClass<'_, B> {
        MidiClass {
            standard_ac: alloc.interface(),
            standard_mc: alloc.interface(),
            standard_bulkout : alloc.bulk(64),
            standard_bulkin: alloc.bulk(64)
        }
    }

    pub fn send(&mut self, data: &[u8;3]) -> Result<usize> {
        //self.write_ep.write(data)
        Ok(0)
    }

}

impl<B: UsbBus> UsbClass<B> for MidiClass<'_, B> {

     fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> Result<()> {
        
        //AUDIO CONTROL STANDARD

        writer.interface(
            self.standard_ac,
            USB_AUDIO_CLASS,
            USB_AUDIOCONTROL_SUBCLASS,
            0 //no protocol,
        )?;

        // AUDIO CONTROL EXTRA INFO
        writer.write(
            CS_INTERFACE,
            &[
                HEADER_SUBTYPE,
                0x00,0x01, // REVISION
                0x09,0x00, //SIZE of class specific descriptions
                0x01, //Number of streaming interfaces
                0x01 // MIDIStreaming interface 1 belongs to this AC interface
            ]
        )?;

        //Streaming Standard

        writer.interface(
            self.standard_mc,
            USB_AUDIO_CLASS,
            USB_MIDISTREAMING_SUBCLASS,
            0, //no protocol
        )?; //Num endpoints?

        //Streaming extra info

        writer.write(
            CS_INTERFACE,
            &[
                MS_HEADER_SUBTYPE,
                0x00,0x01, //REVISION
                0x16,0x00 //Total size of class specific descriptors? (little endian?)
            ]
        )?;
    
        //JACKS

        writer.write(
            CS_INTERFACE,
            &[
                MIDI_IN_JACK_SUBTYPE,
                EMBEDDED,
                0x01, // id
                0x00
            ]
        )?;

        writer.write (
            CS_INTERFACE,
            &[
                MIDI_OUT_JACK_SUBTYPE,
                EMBEDDED,
                0x02,//id
                0x01, // 1 pin
                0x01, // pin 1
                0x01, //sorta vague source pin?
                0x00
            ]
        )?;

        writer.endpoint(&self.standard_bulkout)?;

        writer.write(
            CS_ENDPOINT,
            &[
                MS_GENERAL,
                0x01,
                0x01
            ]
        )?;

        writer.endpoint(&self.standard_bulkin)?;

        writer.write(
            CS_ENDPOINT,
            &[
                MS_GENERAL,
                0x01,
                0x02
            ]
        )?;
        Ok(())
    }

}