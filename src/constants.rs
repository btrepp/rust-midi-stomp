
use usbd_midi::data::{
 byte::u7::U7,
 midi::notes::Note,
 midi::message::Message,
 midi::channel::Channel,
 usb_midi::cable_number::CableNumber,
 usb_midi::usb_midi_event_packet::UsbMidiEventPacket
};


const CABLE :CableNumber = CableNumber::Cable0;
const CHANNEL :Channel= Channel::Channel1;
const NOTE: Note = Note::C3;
const VELOCITY: U7 = U7::MAX;

pub const NOTE_ON : UsbMidiEventPacket = {
    const MIDI :Message= Message::NoteOn(CHANNEL,NOTE,VELOCITY);
    UsbMidiEventPacket{
        cable_number : CABLE,
        message : MIDI
    }
};

pub const NOTE_OFF : UsbMidiEventPacket = {
    const MIDI :Message= Message::NoteOff(CHANNEL,NOTE,VELOCITY);
    UsbMidiEventPacket{
        cable_number : CABLE,
        message : MIDI
    }
};
