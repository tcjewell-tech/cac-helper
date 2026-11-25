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

    let states: Vec<State> = reader_states.iter().map(|rs| rs.event_state()).collect();
    let statuses = reader_states
        .iter()
        .zip(&states)
        .map(|(rs, s)| ReaderStatus {
            name: rs.name().to_string_lossy().into_owned(),
            card_present: s.contains(State::PRESENT),
            state_flags: format!("{:?}", s),
        })
        .collect();

    Ok(DetectionResult {
        outcome: outcome_for(&states),
        readers: statuses,
    })
}

/// One readable card anywhere wins; otherwise a mute (present but
/// unreadable) card; otherwise readers with no card.
fn outcome_for(states: &[State]) -> DetectionOutcome {
    let present = |s: &&State| s.contains(State::PRESENT);
    if states.iter().filter(present).any(|s| !s.contains(State::MUTE)) {
        DetectionOutcome::CardPresent
    } else if states.iter().any(|s| present(&s)) {
        DetectionOutcome::CardUnreadable
    } else {
        DetectionOutcome::ReaderReadyNoCard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(states: &[State]) -> String {
        format!("{:?}", outcome_for(states))
    }

    #[test]
    fn maps_reader_states_to_outcome() {
        let mute = State::PRESENT | State::MUTE;
        assert_eq!(outcome(&[State::EMPTY]), "ReaderReadyNoCard");
        assert_eq!(outcome(&[State::PRESENT]), "CardPresent");
        assert_eq!(outcome(&[mute]), "CardUnreadable");
        // A readable card in any reader beats a mute one in another.
        assert_eq!(outcome(&[mute, State::PRESENT]), "CardPresent");
        // MUTE without PRESENT is not a card.
        assert_eq!(outcome(&[State::MUTE]), "ReaderReadyNoCard");
    }
}
