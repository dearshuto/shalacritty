use std::time::Duration;

use winit::{event::KeyEvent, platform::modifier_supplement::KeyEventExtModifierSupplement};

#[derive(Debug, PartialEq)]
pub enum Action {
    Input(Vec<u8>),
}

pub struct InputEventService {
    receiver: std::sync::mpsc::Receiver<KeyEvent>,
    sender: tokio::sync::mpsc::Sender<Action>,
}

impl InputEventService {
    pub fn new(
        receiver: std::sync::mpsc::Receiver<KeyEvent>,
        sender: tokio::sync::mpsc::Sender<Action>,
    ) -> Self {
        InputEventService { receiver, sender }
    }

    pub async fn serve(mut self) {
        loop {
            match self.receiver.recv_timeout(Duration::from_micros(100)) {
                Ok(request) => self.handle_key_event(request).await,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    '''    async fn handle_key_event(&mut self, event: KeyEvent) {
            if event.state.is_pressed() {
                if let Some(text) = event.text_with_all_modifiers() {
                    let bytes = text.into_bytes();
                    if !bytes.is_empty() {
                        let _ = self.sender.send(Action::Input(bytes)).await;
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use async_trait::async_trait;
        use winit::event::{ElementState, ModifiersState};
        use winit::keyboard::{Key, KeyCode, PhysicalKey};
        use tokio::runtime::Runtime;
        use tokio::sync::mpsc;
        use std::future::Future;

        // A minimal mock for KeyEventExtModifierSupplement for testing purposes.
        // This allows us to control the output of text_with_all_modifiers().
        struct MockKeyEvent {
            state: ElementState,
            mock_text: Option<String>,
        }

        impl KeyEventExtModifierSupplement for MockKeyEvent {
            fn text_with_all_modifiers(&self) -> Option<&str> {
                self.mock_text.as_deref()
            }
        }

        // This is a minimal mock for winit::event::KeyEvent.
        // We only implement the fields/methods that are actually used in `handle_key_event`.
        // It's important not to try to implement everything from winit::event::KeyEvent
        // as it's a complex struct with many internal details.
        impl From<MockKeyEvent> for KeyEvent {
            fn from(mock: MockKeyEvent) -> Self {
                KeyEvent {
                    physical_key: PhysicalKey::Unidentified, // Not used in handle_key_event's logic directly
                    logical_key: Some(Key::Dead), // Not used
                    text: None, // Not used
                    location: winit::keyboard::KeyLocation::Standard, // Not used
                    state: mock.state,
                    repeat: false, // Not used
                    synthetic: false, // Not used
                    modifiers: ModifiersState::empty(), // Not used
                    platform_specific: Default::default(), // Not used
                }
            }
        }

        // Helper to run an async test function
        fn run_test<F>(test_future: F)
        where
            F: Future<Output = ()> + Send + 'static,
        {
            let rt = Runtime::new().unwrap();
            rt.block_on(test_future);
        }

        #[test]
        fn test_handle_key_event_sends_input_on_press() {
            run_test(async {
                let (event_sender, event_receiver) = std::sync::mpsc::channel();
                let (action_sender, mut action_receiver) = mpsc::channel(1);

                let service = InputEventService::new(event_receiver, action_sender);
                let service_handle = tokio::spawn(service.serve());

                let mock_event = MockKeyEvent {
                    state: ElementState::Pressed,
                    mock_text: Some("a".to_string()),
                };
                event_sender.send(mock_event.into()).unwrap();

                // Wait for the action to be processed
                tokio::time::sleep(Duration::from_millis(10)).await;

                let received_action = action_receiver.try_recv().unwrap();
                assert_eq!(received_action, Action::Input(vec![b'a']));

                // Shut down the service
                drop(event_sender);
                service_handle.await.unwrap();
            });
        }

        #[test]
        fn test_handle_key_event_sends_control_sequence() {
            run_test(async {
                let (event_sender, event_receiver) = std::sync::mpsc::channel();
                let (action_sender, mut action_receiver) = mpsc::channel(1);

                let service = InputEventService::new(event_receiver, action_sender);
                let service_handle = tokio::spawn(service.serve());

                // Simulate Ctrl+C, which typically yields ``
                let mock_event = MockKeyEvent {
                    state: ElementState::Pressed,
                    mock_text: Some("".to_string()), // This is what winit::event::KeyEvent would ideally produce
                };
                event_sender.send(mock_event.into()).unwrap();

                tokio::time::sleep(Duration::from_millis(10)).await;

                let received_action = action_receiver.try_recv().unwrap();
                assert_eq!(received_action, Action::Input(vec![0x03]));

                drop(event_sender);
                service_handle.await.unwrap();
            });
        }

        #[test]
        fn test_handle_key_event_ignores_release_events() {
            run_test(async {
                let (event_sender, event_receiver) = std::sync::mpsc::channel();
                let (action_sender, mut action_receiver) = mpsc::channel(1);

                let service = InputEventService::new(event_receiver, action_sender);
                let service_handle = tokio::spawn(service.serve());

                let mock_event = MockKeyEvent {
                    state: ElementState::Released, // Released state
                    mock_text: Some("b".to_string()),
                };
                event_sender.send(mock_event.into()).unwrap();

                tokio::time::sleep(Duration::from_millis(10)).await;

                // Ensure no action was sent
                assert!(action_receiver.try_recv().is_err());

                drop(event_sender);
                service_handle.await.unwrap();
            });
        }

        #[test]
        fn test_handle_key_event_ignores_empty_text() {
            run_test(async {
                let (event_sender, event_receiver) = std::sync::mpsc::channel();
                let (action_sender, mut action_receiver) = mpsc::channel(1);

                let service = InputEventService::new(event_receiver, action_sender);
                let service_handle = tokio::spawn(service.serve());

                let mock_event = MockKeyEvent {
                    state: ElementState::Pressed,
                    mock_text: Some("".to_string()), // Empty text
                };
                event_sender.send(mock_event.into()).unwrap();

                tokio::time::sleep(Duration::from_millis(10)).await;

                // Ensure no action was sent
                assert!(action_receiver.try_recv().is_err());

                drop(event_sender);
                service_handle.await.unwrap();
            });
        }

        #[test]
        fn test_handle_key_event_ignores_none_text() {
            run_test(async {
                let (event_sender, event_receiver) = std::sync::mpsc::channel();
                let (action_sender, mut action_receiver) = mpsc::channel(1);

                let service = InputEventService::new(event_receiver, action_sender);
                let service_handle = tokio::spawn(service.serve());

                let mock_event = MockKeyEvent {
                    state: ElementState::Pressed,
                    mock_text: None, // None text
                };
                event_sender.send(mock_event.into()).unwrap();

                tokio::time::sleep(Duration::from_millis(10)).await;

                // Ensure no action was sent
                assert!(action_receiver.try_recv().is_err());

                drop(event_sender);
                service_handle.await.unwrap();
            });
        }
    }
    ''
