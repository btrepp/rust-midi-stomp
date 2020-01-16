use heapless::consts::U5;
use heapless::LinearMap;
use usbd_midi::data::byte::u7::U7;
use usbd_midi::data::midi::channel::Channel;
use usbd_midi::data::midi::message::Message as MidiMessage;
use usbd_midi::data::midi::notes::Note;
use usbd_midi::data::usb_midi::cable_number::CableNumber;
use usbd_midi::data::usb_midi::usb_midi_event_packet::UsbMidiEventPacket;

/// The buttons the user can press
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Button {
    One,
    Two,
    Three,
    Four,
    Five,
}

type ButtonMap = LinearMap<Button, State, U5>;

/// States the buttons emit
#[derive(Clone, Copy, PartialEq, Eq)]
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

impl From<Button> for Note {
    fn from(button:Button) -> Note {
        match button {
            Button::One => Note::C3,
            Button::Two => Note::Cs3,
            Button::Three => Note::D3,
            Button::Four => Note::Ds3,
            Button::Five => Note::E3,
        }
    }
}

/// A button message, this is the event of the button
/// and it's current state
pub type Message = (Button, State);

#[derive(Clone)]
/// The application state
pub struct ApplicationState {
    buttons: ButtonMap,
    cable: CableNumber,
    channel: Channel,
}

/// Converts a button press into a usb midi packet
fn message_to_midi(
    cable: CableNumber,
    channel: Channel,
    message: Message,
) -> UsbMidiEventPacket {
    const VELOCITY: U7 = U7::MAX;
    let (button, direction) = message;
    let note = button.into();
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
/// and calculates the midi events emitted transitioning between the two
/// states. Note: if a -> b -> c and called with a,c some state transitions may 
/// be missed
pub fn midi_events<'a>(
    old_application: &'a ApplicationState,
    new_application: &'a ApplicationState,
) -> impl Iterator<Item = UsbMidiEventPacket> + 'a {
    let compare = move | (button,value):(&Button,&State) | -> Option<UsbMidiEventPacket> {
        let find = old_application.buttons.get(&button);
        let midi: UsbMidiEventPacket = message_to_midi(
            new_application.cable,
            new_application.channel,
            (*button,*value));
        match find {
            Some(old_value) if *old_value != *value => Some (midi),
            _ => None
        }  
    };
    let events = new_application.buttons.iter().filter_map(compare);
    events
}

impl ApplicationState {

    /// Initializes a default application state
    /// all buttons are off
    pub fn init() -> ApplicationState {
        let mut map = LinearMap(heapless::i::LinearMap::new());
        let _ = map.insert(Button::One, State::Off);
        let _ = map.insert(Button::Two, State::Off);
        let _ = map.insert(Button::Three, State::Off);
        let _ = map.insert(Button::Four, State::Off);
        let _ = map.insert(Button::Five, State::Off);
        ApplicationState {
            buttons: map,
            cable: CableNumber::Cable1,
            channel: Channel::Channel1,
        }
    }

    /// Updates the button state. TEA like
    pub fn update(&mut self, message: Message) -> () {
        let (button, direction) = message;

        let current = self.buttons.get(&button);
        match current {
            Some(state) if *state != direction => {
                let _ = self.buttons.insert(button, direction);
            }
            _ => (),
        }
    }
}
