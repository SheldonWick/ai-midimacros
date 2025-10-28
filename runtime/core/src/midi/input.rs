use midir::{Ignore, MidiInput};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::executor::MidiEvent;

#[derive(Debug)]
pub struct MidiHandle {
    pub join_handle: JoinHandle<()>,
}

pub fn spawn_midi_listener<T: Into<String>>(
    client_name: T,
    sender: broadcast::Sender<MidiEvent>,
) -> anyhow::Result<MidiHandle> {
    let client_name = client_name.into();
    let mut input = MidiInput::new(client_name.as_str())?;
    input.ignore(Ignore::None);

    let ports = input.ports();
    if ports.is_empty() {
        anyhow::bail!("No MIDI input ports available");
    }
    let port = ports[0].clone();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<MidiEvent>(32);

    std::thread::spawn(move || {
        let input = input;
        let _connection = input
            .connect(
                &port,
                "ai-midimacros",
                move |_, message, _| {
                    if message.len() >= 2 {
                        let status = message[0] & 0xF0;
                        if status == 0x90 && message.len() >= 3 {
                            let _ = tx.blocking_send(MidiEvent {
                                note: message[1],
                                velocity: message[2],
                            });
                        }
                    }
                },
                (),
            )
            .expect("Failed to open MIDI input");
        loop {
            std::thread::park();
        }
    });

    let join_handle = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let _ = sender.send(event);
        }
    });

    Ok(MidiHandle { join_handle })
}
