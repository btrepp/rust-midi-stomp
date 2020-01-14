use usbd_midi::data::byte::u7::U7;
use usbd_midi::data::midi::channel::Channel;
use usbd_midi::data::midi::message::Message as MidiMessage;
use usbd_midi::data::midi::notes::Note;
use usbd_midi::data::usb_midi::cable_number::CableNumber;
use usbd_midi::data::usb_midi::usb_midi_event_packet::UsbMidiEventPacket;

/// The buttons the user can press
#[derive(Clone, Copy, PartialEq)]
pub enum Button {
    One,
    Two,
    Three,
    Four,
    Five,
}

/// States the buttons emit
#[derive(Clone, Copy, PartialEq)]
pub enum State {
    On,
    Off,
}

impl From<bool> for State {
    fn from(value: bool) -> State {
        match value {
            true => State::On,
            false => State::Off,
        }
    }
}

/// A button message, this is the event of the button
/// and it's current state
pub type Message = (Button, State);

/// The effects that can be omitted.
/// Single Note for a single event
/// Transition may cover changing from one note to the other
pub enum Effect {
    Midi(UsbMidiEventPacket),
    Nothing,
}

#[derive(Copy, Clone)]
/// The application state
pub struct ApplicationState {
    button1: State,
    button2: State,
    button3: State,
    button4: State,
    button5: State,
    cable: CableNumber,
    channel: Channel,
}

fn button_to_note(button: Button) -> Note {
    match button {
        Button::One => Note::C3,
        Button::Two => Note::Cs3,
        Button::Three => Note::D3,
        Button::Four => Note::Ds3,
        Button::Five => Note::E3,
    }
}

fn message_to_midi(
    cable: CableNumber,
    channel: Channel,
    message: Message,
) -> UsbMidiEventPacket {
    const VELOCITY: U7 = U7::MAX;
    let (button, direction) = message;
    let note = button_to_note(button);
    match direction {
        State::On => {
            let midi = MidiMessage::NoteOn(channel, note, VELOCITY);
            UsbMidiEventPacket::from_midi(cable, midi)
        }
        State::Off => {
            let midi = MidiMessage::NoteOff(channel, note, VELOCITY);
            UsbMidiEventPacket::from_midi(cable, midi)
        }
    }
}

impl ApplicationState {
    pub fn init() -> ApplicationState {
        ApplicationState {
            button1: State::Off,
            button2: State::Off,
            button3: State::Off,
            button4: State::Off,
            button5: State::Off,
            cable: CableNumber::Cable1,
            channel: Channel::Channel1,
        }
    }

    pub fn update(
        current: ApplicationState,
        message: Message,
    ) -> (ApplicationState, Effect) {
        let (button, direction) = message;
        let midi = message_to_midi(current.cable, current.channel, message);
        let mut updated_state = current;

        let update = |button:&mut State| -> Effect {
            if *button == direction {
                Effect::Nothing
            } else {
                *button = direction;         
                Effect::Midi(midi)
            }
        };

        let effect = 
            match button {
                Button::One => update(&mut updated_state.button1),
                Button::Two => update(&mut updated_state.button2),
                Button::Three => update(&mut updated_state.button3),
                Button::Four => update(&mut updated_state.button4),
                Button::Five => update(&mut updated_state.button5)
            };
        (updated_state,effect)
    }
}
