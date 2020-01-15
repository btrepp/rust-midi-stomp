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

/// Takes a old state and a new state
/// and calculates the midi events emitted transitioning between the to
/// Note that this has an upper bound
/// of 5 events.
pub fn midi_events<'a>(
    old_application: &ApplicationState,
    new_application: &ApplicationState,
) -> impl Iterator<Item = UsbMidiEventPacket> {
    let check = |old: State,
                 new: State,
                 button: Button|
     -> Option<UsbMidiEventPacket> {
        if old == new {
            None
        } else {
            let message = (button, new);
            let midi: UsbMidiEventPacket = message_to_midi(
                new_application.cable,
                new_application.channel,
                message,
            );
            Some(midi)
        }
    };
    let mut result =
        heapless::Vec::<Option<UsbMidiEventPacket>, heapless::consts::U5>::new(
        );
    let _ = result.push(check(
        old_application.button1,
        new_application.button1,
        Button::One,
    ));
    let _ = result.push(check(
        old_application.button2,
        new_application.button2,
        Button::Two,
    ));
    let _ = result.push(check(
        old_application.button3,
        new_application.button3,
        Button::Three,
    ));
    let _ = result.push(check(
        old_application.button4,
        new_application.button4,
        Button::Four,
    ));
    let _ = result.push(check(
        old_application.button5,
        new_application.button5,
        Button::Five,
    ));
    result.into_iter().filter_map(|x| x)
}

impl ApplicationState {
    pub const INIT: ApplicationState = ApplicationState {
        button1: State::Off,
        button2: State::Off,
        button3: State::Off,
        button4: State::Off,
        button5: State::Off,
        cable: CableNumber::Cable1,
        channel: Channel::Channel1,
    };

    pub fn update(mut self, message: Message) -> () {
        let (button, direction) = message;

        let update = |button: &mut State| -> () {
            *button = direction;
        };

        match button {
            Button::One => update(&mut self.button1),
            Button::Two => update(&mut self.button2),
            Button::Three => update(&mut self.button3),
            Button::Four => update(&mut self.button4),
            Button::Five => update(&mut self.button5),
        };
    }
}
