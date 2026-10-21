use pcsc::*;
use serde::Serialize;

/// Status of a single connected smart card reader.
#[derive(Serialize, Clone, Debug)]
pub struct ReaderStatus {
    pub name: String,
    pub card_present: bool,
    /// Raw PC/SC state flags, useful for diagnostics UI ("Unpowered",
    /// "Mute" = card present but unreadable, etc.)
    pub state_flags: String,
}

/// High-level result the frontend can branch its guidance on.
#[derive(Serialize, Clone, Debug)]
pub enum DetectionOutcome {
    /// No reader plugged in / recognized by the OS at all.
    NoReaderFound,
    /// Reader found, but no card inserted.
    ReaderReadyNoCard,
    /// Reader found, card inserted, but PC/SC can't read it
    /// (usually a driver/middleware problem, not a card problem).
    CardUnreadable,
    /// Reader found, card inserted, and readable at the PC/SC layer.
    CardPresent,
}

#[derive(Serialize, Clone, Debug)]
pub struct DetectionResult {
    pub outcome: DetectionOutcome,
    pub readers: Vec<ReaderStatus>,
}

/// Enumerate connected readers and their current card status.
///
/// This only proves the OS-level PC/SC stack sees the hardware. It does
/// NOT prove middleware or DoD certs are installed correctly -- that's
/// module 2/3's job. Keeping this function scoped to detection only makes
/// it easy to give the user an accurate "which layer is broken" message.
pub fn detect_readers() -> Result<DetectionResult, Box<dyn std::error::Error>> {
    let ctx = match Context::establish(Scope::User) {
        Ok(c) => c,
        Err(Error::NoService) => {
            // PC/SC service itself isn't running (common on fresh Windows
            // installs where "Smart Card" service is set to manual/off).
            return Ok(DetectionResult {
                outcome: DetectionOutcome::NoReaderFound,
                readers: vec![],
            });
        }
        Err(e) => return Err(Box::new(e)),
    };

    let mut readers_buf = [0u8; 2048];
    let reader_names: Vec<_> = match ctx.list_readers(&mut readers_buf) {
        Ok(iter) => iter.collect(),
        Err(Error::NoReadersAvailable) => vec![],
        Err(e) => return Err(Box::new(e)),
    };

    if reader_names.is_empty() {
        return Ok(DetectionResult {
            outcome: DetectionOutcome::NoReaderFound,
            readers: vec![],
        });
    }

    let mut reader_states: Vec<ReaderState> = reader_names
        .iter()
        .map(|name| ReaderState::new(*name, State::UNAWARE))
        .collect();

    // Ask PC/SC for current status of each reader (0ms timeout = don't
    // block waiting for a state change, just report what's true right now).
    ctx.get_status_change(std::time::Duration::from_millis(0), &mut reader_states)
        .or_else(|e| if e == Error::Timeout { Ok(()) } else { Err(e) })?;

    let mut statuses = Vec::new();
    let mut any_card_present = false;
    let mut any_card_unreadable = false;

    for rs in &reader_states {
        let event_state = rs.event_state();
        let card_present = event_state.contains(State::PRESENT);
        let mute = event_state.contains(State::MUTE);

        if card_present && mute {
            any_card_unreadable = true;
        } else if card_present {
            any_card_present = true;
        }

        statuses.push(ReaderStatus {
            name: rs.name().to_string_lossy().into_owned(),
            card_present,
            state_flags: format!("{:?}", event_state),
        });
    }

    let outcome = if any_card_present {
        DetectionOutcome::CardPresent
    } else if any_card_unreadable {
        DetectionOutcome::CardUnreadable
    } else {
        DetectionOutcome::ReaderReadyNoCard
    };

    Ok(DetectionResult {
        outcome,
        readers: statuses,
    })
}
