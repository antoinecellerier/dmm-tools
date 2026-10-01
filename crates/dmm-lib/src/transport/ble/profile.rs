//! The GATT profiles a peer's byte stream can ride on, which one a peer's
//! services carry, and how to write to it.

use super::brymen::BRYMEN;
use super::eevblog121gw::EEVBLOG_121GW;
use super::fff0::FFF0;
use super::issc::ISSC_UART;
use super::owon::OWON;
use btleplug::api::{CharPropFlags, Characteristic, WriteType};
use std::collections::BTreeSet;

/// Write size when the platform has not negotiated an MTU yet: the BLE
/// default ATT MTU of 23 bytes minus the three-byte write header.
const DEFAULT_WRITE_CHUNK: usize = 20;

/// A GATT layout a peer carries its byte stream on. UUIDs are lowercase:
/// btleplug renders them that way, and the comparison is textual.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct GattProfile {
    /// What the log calls it.
    pub(super) name: &'static str,
    pub(super) service: &'static str,
    /// Meter → host: its notifications carry the stream.
    pub(super) notify: &'static str,
    /// Host → meter: commands are written here.
    pub(super) write: &'static str,
    /// The properties the notify characteristic must offer one of for the
    /// profile to be taken; empty asks nothing.
    pub(super) notify_needs: CharPropFlags,
    /// The same for the write characteristic.
    pub(super) write_needs: CharPropFlags,
    /// Whether every write goes unacknowledged, whatever the
    /// characteristic lists ([`write_type`]).
    pub(super) always_unacknowledged: bool,
    /// What runs between discovery and the subscribe.
    pub(super) bring_up: BringUp,
    /// The smallest ATT MTU the peer's notifications fit whole in, when one
    /// is known: under it the open warns.
    pub(super) min_mtu: Option<u16>,
    /// Whether UNI-T's adapter heartbeat is taken off the stream.
    pub(super) strips_adapter_heartbeat: bool,
}

/// What a profile's bring-up does before the subscribe.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum BringUp {
    /// Nothing: subscribe and go.
    Subscribe,
    /// The BM78xBT's password login (`brymen.rs`).
    BrymenLogin,
    /// Read this characteristic, in the profile's service, once before the
    /// subscribe, and hand its value to the protocol
    /// ([`crate::transport::Transport::info_characteristic`]). The profile
    /// is taken only where the characteristic offers a read.
    ReadInfo(&'static str),
}

/// The profiles, in the order a peer's services are tried
/// ([`choose_profile`]).
///
/// ISSC first, as before there was a second profile: no known ISSC peer has
/// any other of these services
/// (`docs/research/ut-d07b/reverse-engineered-protocol.md` §2), and one that
/// carried any would be read over ISSC. Then the meters'
/// own services, the 121GW's and Brymen's, whose UUIDs alone say which meter
/// it is; the order between those two never meets a real peer. FFF0 last:
/// it is a common service on generic modules, so a peer that also carries a
/// meter's own service is read over that.
///
/// OWON's after it, in the same service: it is taken only by a peer whose
/// FFF4 offers no write, which FFF0 refuses, so every peer that was read
/// over a profile before still is. A peer whose FFF4 takes writes is read
/// over FFF0 even with OWON's other characteristics beside it.
pub(super) const PROFILES: [&GattProfile; 5] = [&ISSC_UART, &EEVBLOG_121GW, &BRYMEN, &FFF0, &OWON];

/// A profile's characteristics, as one look at a peer's services found them.
pub(super) struct Chosen {
    pub(super) profile: &'static GattProfile,
    pub(super) notify: Characteristic,
    pub(super) write: Characteristic,
    /// The characteristic a [`BringUp::ReadInfo`] profile reads; `None` for
    /// every other profile.
    pub(super) info: Option<Characteristic>,
}

/// The profile a peer's discovered characteristics carry, or the role
/// (`"write"` or `"notify"`) that is missing.
///
/// The first of [`PROFILES`] whose two characteristics are there, under its
/// service, and offer what the profile needs of them, with the readable
/// characteristic a [`BringUp::ReadInfo`] names beside them. Failing that, the
/// error names what the closest profile lacks: the first whose service
/// carries either of its characteristics, else ISSC, which is what a peer
/// with no known profile always heard.
pub(super) fn choose_profile(
    characteristics: &BTreeSet<Characteristic>,
) -> std::result::Result<Chosen, &'static str> {
    let find = |service: &str, uuid: &str| {
        characteristics
            .iter()
            .find(|c| c.service_uuid.to_string() == service && c.uuid.to_string() == uuid)
    };
    let found = |profile: &GattProfile| {
        (
            find(profile.service, profile.notify),
            find(profile.service, profile.write),
        )
    };

    for profile in PROFILES {
        let (Some(notify), Some(write)) = found(profile) else {
            continue;
        };
        if !fits(notify.properties, profile.notify_needs)
            || !fits(write.properties, profile.write_needs)
        {
            continue;
        }
        let info = match profile.bring_up {
            BringUp::ReadInfo(uuid) => match find(profile.service, uuid) {
                Some(info) if info.properties.contains(CharPropFlags::READ) => Some(info.clone()),
                _ => continue,
            },
            BringUp::Subscribe | BringUp::BrymenLogin => None,
        };
        return Ok(Chosen {
            profile,
            notify: notify.clone(),
            write: write.clone(),
            info,
        });
    }

    let closest = PROFILES
        .into_iter()
        .find(|profile| {
            let (notify, write) = found(profile);
            notify.is_some() || write.is_some()
        })
        .unwrap_or(PROFILES[0]);
    let (_, write) = found(closest);
    let writable = write.is_some_and(|c| fits(c.properties, closest.write_needs));
    Err(if writable { "notify" } else { "write" })
}

/// Whether a characteristic with `properties` offers any of `needs`; empty
/// `needs` asks nothing.
fn fits(properties: CharPropFlags, needs: CharPropFlags) -> bool {
    needs.is_empty() || properties.intersects(needs)
}

/// How many bytes fit in one write at `mtu`: the ATT MTU less the three-byte
/// write header, and the BLE default when the platform has not negotiated one.
pub(super) fn write_chunk_size(mtu: u16) -> usize {
    match (mtu as usize).checked_sub(3) {
        Some(0) | None => DEFAULT_WRITE_CHUNK,
        Some(n) => n,
    }
}

/// How to write to `write_char`: unacknowledged wherever the peer takes it.
///
/// An acknowledged write costs a round trip per poll (0.8 s against 0.63 s
/// per reading on our adapter), and a dead link is caught by `read_timeout`
/// instead. A profile that says so, ISSC, is always written that way. Any
/// other characteristic that lists only acknowledged writes gets those, as
/// ZOTEK's current app and both 121GW apps write with the characteristic's
/// own type (`docs/research/zotek/reverse-engineered-protocol.md` §2;
/// `docs/research/121gw/reverse-engineered-protocol.md` §2). One that lists
/// no write at all is still sent an unacknowledged one, for the platform or
/// the peer to refuse.
pub(super) fn write_type(profile: &GattProfile, write_char: &Characteristic) -> WriteType {
    let props = write_char.properties;
    if profile.always_unacknowledged
        || props.contains(CharPropFlags::WRITE_WITHOUT_RESPONSE)
        || !props.contains(CharPropFlags::WRITE)
    {
        WriteType::WithoutResponse
    } else {
        WriteType::WithResponse
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One characteristic as discovery reports it.
    fn characteristic(service: &str, uuid: &str, properties: CharPropFlags) -> Characteristic {
        Characteristic {
            uuid: uuid.parse().unwrap(),
            service_uuid: service.parse().unwrap(),
            properties,
            descriptors: BTreeSet::new(),
        }
    }

    /// The UART characteristics with the flags the UT-D07B lists
    /// (`docs/research/ut-d07b/reverse-engineered-protocol.md` §2).
    fn issc() -> [Characteristic; 2] {
        [
            characteristic(ISSC_UART.service, ISSC_UART.notify, CharPropFlags::NOTIFY),
            characteristic(
                ISSC_UART.service,
                ISSC_UART.write,
                CharPropFlags::WRITE | CharPropFlags::WRITE_WITHOUT_RESPONSE,
            ),
        ]
    }

    /// FFF4 with `properties`.
    fn fff4(properties: CharPropFlags) -> Characteristic {
        characteristic(FFF0.service, FFF0.notify, properties)
    }

    /// FFF4 with the flags a ZT-5B lists
    /// (`docs/research/zotek/reverse-engineered-protocol.md` §11.4).
    fn fff4_as_listed() -> Characteristic {
        fff4(CharPropFlags::READ | CharPropFlags::WRITE_WITHOUT_RESPONSE | CharPropFlags::NOTIFY)
    }

    /// The 121GW's data characteristic with `properties`.
    fn gw(properties: CharPropFlags) -> Characteristic {
        characteristic(EEVBLOG_121GW.service, EEVBLOG_121GW.notify, properties)
    }

    /// Brymen's reading and command characteristics with the properties r4
    /// lists (`docs/research/bm78xbt/reverse-engineered-protocol.md` §2).
    fn brymen() -> [Characteristic; 2] {
        [
            characteristic(BRYMEN.service, BRYMEN.notify, CharPropFlags::NOTIFY),
            characteristic(
                BRYMEN.service,
                BRYMEN.write,
                CharPropFlags::READ | CharPropFlags::WRITE,
            ),
        ]
    }

    /// OWON's information, key and stream characteristics, the information
    /// one with `info`: FFF4 notifies only, as a B35T+ lists it, and FFF3
    /// takes both writes (`docs/research/owon/reverse-engineered-protocol.md`
    /// §14.4).
    fn owon(info: CharPropFlags) -> [Characteristic; 3] {
        let BringUp::ReadInfo(info_uuid) = OWON.bring_up else {
            panic!("OWON's profile reads its information characteristic");
        };
        [
            characteristic(OWON.service, info_uuid, info),
            characteristic(
                OWON.service,
                OWON.write,
                CharPropFlags::WRITE | CharPropFlags::WRITE_WITHOUT_RESPONSE,
            ),
            characteristic(OWON.service, OWON.notify, CharPropFlags::NOTIFY),
        ]
    }

    /// The name of the profile `characteristics` pick, or the missing role.
    fn choose(
        characteristics: impl IntoIterator<Item = Characteristic>,
    ) -> std::result::Result<&'static str, &'static str> {
        choose_profile(&characteristics.into_iter().collect()).map(|c| c.profile.name)
    }

    /// A peer with ISSC is read over it, even beside an FFF0 service, with
    /// the TX characteristic subscribed and the RX one written.
    #[test]
    fn issc_is_chosen_first() {
        assert_eq!(choose(issc()), Ok(ISSC_UART.name));
        assert_eq!(
            choose(issc().into_iter().chain([fff4_as_listed()])),
            Ok(ISSC_UART.name)
        );

        let chosen = choose_profile(&issc().into_iter().collect()).unwrap();
        assert_eq!(chosen.notify.uuid.to_string(), ISSC_UART.notify);
        assert_eq!(chosen.write.uuid.to_string(), ISSC_UART.write);
    }

    /// A peer with FFF0 alone is read over FFF4 both ways, whichever write
    /// it takes.
    #[test]
    fn fff0_is_chosen_without_issc() {
        assert_eq!(choose([fff4_as_listed()]), Ok(FFF0.name));
        assert_eq!(
            choose([fff4(CharPropFlags::WRITE | CharPropFlags::NOTIFY)]),
            Ok(FFF0.name)
        );

        let chosen = choose_profile(&[fff4_as_listed()].into_iter().collect()).unwrap();
        assert_eq!(chosen.notify.uuid.to_string(), FFF0.notify);
        assert_eq!(chosen.write, chosen.notify);
    }

    /// An FFF4 that only indicates still carries the stream: btleplug
    /// subscribes to whichever the characteristic offers.
    #[test]
    fn fff4_may_indicate_instead_of_notify() {
        assert_eq!(
            choose([fff4(
                CharPropFlags::WRITE_WITHOUT_RESPONSE | CharPropFlags::INDICATE
            )]),
            Ok(FFF0.name)
        );
    }

    /// Writes go unacknowledged wherever the peer takes that: always on
    /// ISSC, and on an FFF4 unless it lists acknowledged writes alone.
    #[test]
    fn write_type_follows_what_fff4_takes() {
        let [_, issc_write] = issc();
        assert_eq!(
            write_type(&ISSC_UART, &issc_write),
            WriteType::WithoutResponse
        );
        assert_eq!(
            write_type(&FFF0, &fff4_as_listed()),
            WriteType::WithoutResponse
        );
        assert_eq!(
            write_type(&FFF0, &fff4(CharPropFlags::WRITE | CharPropFlags::NOTIFY)),
            WriteType::WithResponse
        );
    }

    /// An FFF4 that cannot carry the stream both ways is no profile, and the
    /// error names what it lacks.
    #[test]
    fn fff4_needs_notify_and_a_write() {
        assert_eq!(
            choose([fff4(CharPropFlags::READ | CharPropFlags::NOTIFY)]),
            Err("write")
        );
        assert_eq!(
            choose([fff4(
                CharPropFlags::READ | CharPropFlags::WRITE_WITHOUT_RESPONSE
            )]),
            Err("notify")
        );
        assert_eq!(choose([fff4(CharPropFlags::READ)]), Err("write"));
    }

    /// A peer with neither profile fails as it did with ISSC alone; a known
    /// characteristic under another service does not count.
    #[test]
    fn a_peer_with_neither_profile_is_refused() {
        assert_eq!(choose([]), Err("write"));
        assert_eq!(
            choose([characteristic(
                "0000180a-0000-1000-8000-00805f9b34fb",
                FFF0.notify,
                CharPropFlags::WRITE | CharPropFlags::NOTIFY,
            )]),
            Err("write")
        );
        let [notify, write] = issc();
        assert_eq!(choose([notify]), Err("write"));
        assert_eq!(choose([write]), Err("notify"));
    }

    /// A peer with the 121GW's service is read over its one characteristic
    /// both ways.
    #[test]
    fn the_121gw_service_is_chosen_without_issc() {
        let data = gw(CharPropFlags::WRITE | CharPropFlags::INDICATE);
        assert_eq!(choose([data.clone()]), Ok(EEVBLOG_121GW.name));

        let chosen = choose_profile(&[data].into_iter().collect()).unwrap();
        assert_eq!(chosen.notify.uuid.to_string(), EEVBLOG_121GW.notify);
        assert_eq!(chosen.write, chosen.notify);
    }

    /// The stream is what the 121GW's characteristic has to carry, by
    /// indication or notification; which one the meter offers and which
    /// write it takes are not known (spec §2), so neither decides.
    #[test]
    fn the_121gw_may_notify_or_indicate() {
        for properties in [
            CharPropFlags::INDICATE,
            CharPropFlags::NOTIFY,
            CharPropFlags::WRITE | CharPropFlags::INDICATE,
            CharPropFlags::WRITE_WITHOUT_RESPONSE | CharPropFlags::NOTIFY,
        ] {
            assert_eq!(
                choose([gw(properties)]),
                Ok(EEVBLOG_121GW.name),
                "{properties:?}"
            );
        }
    }

    /// ISSC still comes first beside the 121GW's service.
    #[test]
    fn issc_outranks_the_121gw() {
        let data = gw(CharPropFlags::WRITE | CharPropFlags::INDICATE);
        assert_eq!(choose(issc().into_iter().chain([data])), Ok(ISSC_UART.name));
    }

    /// The 121GW's service is the meter's own and FFF0 is generic, so a peer
    /// with both is read as a 121GW.
    #[test]
    fn the_121gw_outranks_fff0() {
        let data = gw(CharPropFlags::WRITE | CharPropFlags::INDICATE);
        assert_eq!(choose([fff4_as_listed(), data]), Ok(EEVBLOG_121GW.name));
    }

    /// A 121GW characteristic that can neither notify nor indicate carries
    /// no stream, and the error says so.
    #[test]
    fn a_121gw_characteristic_that_cannot_stream_is_refused() {
        assert_eq!(
            choose([gw(CharPropFlags::READ | CharPropFlags::WRITE)]),
            Err("notify")
        );
    }

    /// The 121GW's characteristic is written the way it takes, as FFF4 is:
    /// unacknowledged unless it lists acknowledged writes alone.
    #[test]
    fn write_type_follows_what_the_121gw_characteristic_takes() {
        assert_eq!(
            write_type(
                &EEVBLOG_121GW,
                &gw(CharPropFlags::WRITE | CharPropFlags::INDICATE)
            ),
            WriteType::WithResponse
        );
        assert_eq!(
            write_type(
                &EEVBLOG_121GW,
                &gw(CharPropFlags::WRITE_WITHOUT_RESPONSE
                    | CharPropFlags::WRITE
                    | CharPropFlags::INDICATE)
            ),
            WriteType::WithoutResponse
        );
        // No write listed: the write goes out unacknowledged and is refused.
        assert_eq!(
            write_type(&EEVBLOG_121GW, &gw(CharPropFlags::INDICATE)),
            WriteType::WithoutResponse
        );
    }

    /// A peer with Brymen's service is read on its reading characteristic
    /// and written on its command one.
    #[test]
    fn brymens_service_is_chosen_without_issc() {
        assert_eq!(choose(brymen()), Ok(BRYMEN.name));

        let chosen = choose_profile(&brymen().into_iter().collect()).unwrap();
        assert_eq!(chosen.notify.uuid.to_string(), BRYMEN.notify);
        assert_eq!(chosen.write.uuid.to_string(), BRYMEN.write);
    }

    /// The command characteristic has to take an acknowledged write, the
    /// only kind the login sends; being readable is left to the login.
    #[test]
    fn brymens_command_characteristic_needs_an_acknowledged_write() {
        let [notify, _] = brymen();
        let write_only = characteristic(BRYMEN.service, BRYMEN.write, CharPropFlags::WRITE);
        assert_eq!(choose([notify.clone(), write_only]), Ok(BRYMEN.name));

        let unacknowledged = characteristic(
            BRYMEN.service,
            BRYMEN.write,
            CharPropFlags::READ | CharPropFlags::WRITE_WITHOUT_RESPONSE,
        );
        assert_eq!(choose([notify, unacknowledged]), Err("write"));
    }

    /// The readings come by notification (spec §2); a reading
    /// characteristic that cannot notify carries no stream.
    #[test]
    fn brymens_reading_characteristic_has_to_notify() {
        let [_, write] = brymen();
        let indicates = characteristic(BRYMEN.service, BRYMEN.notify, CharPropFlags::INDICATE);
        assert_eq!(choose([indicates, write]), Err("notify"));
    }

    /// Half of Brymen's service is no profile, and the error names the half
    /// that is missing.
    #[test]
    fn half_of_brymens_service_is_refused() {
        let [notify, write] = brymen();
        assert_eq!(choose([notify]), Err("write"));
        assert_eq!(choose([write]), Err("notify"));
    }

    /// ISSC and the 121GW still come first beside Brymen's service; the
    /// order among the meters' own services never meets a real peer.
    #[test]
    fn issc_and_the_121gw_outrank_brymen() {
        assert_eq!(
            choose(issc().into_iter().chain(brymen())),
            Ok(ISSC_UART.name)
        );
        let data = gw(CharPropFlags::WRITE | CharPropFlags::INDICATE);
        assert_eq!(
            choose(brymen().into_iter().chain([data])),
            Ok(EEVBLOG_121GW.name)
        );
    }

    /// Brymen's service is the meter's own and FFF0 is generic, so a peer
    /// with both is read as a BM78xBT.
    #[test]
    fn brymen_outranks_fff0() {
        assert_eq!(
            choose(brymen().into_iter().chain([fff4_as_listed()])),
            Ok(BRYMEN.name)
        );
    }

    /// A peer whose FFF4 only notifies, with a readable FFF2 and a writable
    /// FFF3 beside it, is read over OWON's profile: subscribed on FFF4,
    /// written on FFF3, and FFF2 read first.
    #[test]
    fn owon_is_chosen_when_fff4_only_notifies() {
        let [info, keys, stream] = owon(CharPropFlags::READ);
        let chosen = choose_profile(&[info.clone(), keys.clone(), stream.clone()].into()).unwrap();
        assert_eq!(chosen.profile.name, OWON.name);
        assert_eq!(chosen.notify, stream);
        assert_eq!(chosen.write, keys);
        assert_eq!(chosen.info, Some(info));
    }

    /// ZOTEK's FFF0 holds FFF4 alone, taking writes, and stays on FFF0's
    /// profile with nothing read first.
    #[test]
    fn a_zotek_fff0_peer_stays_on_fff0() {
        let chosen = choose_profile(&[fff4_as_listed()].into()).unwrap();
        assert_eq!(chosen.profile.name, FFF0.name);
        assert_eq!(chosen.info, None);
    }

    /// FFF0 comes first: a peer whose FFF4 takes writes is read over it even
    /// with OWON's other characteristics there, as it was before OWON's
    /// profile.
    #[test]
    fn fff0_keeps_a_peer_whose_fff4_takes_writes() {
        let [info, keys, _] = owon(CharPropFlags::READ);
        let chosen = choose_profile(&[info, keys, fff4_as_listed()].into()).unwrap();
        assert_eq!(chosen.profile.name, FFF0.name);
        assert_eq!(chosen.write, fff4_as_listed());
        assert_eq!(chosen.info, None);
    }

    /// Without a readable FFF2 there is no OWON profile, and FFF0, the
    /// closest, names the write it lacks.
    #[test]
    fn owon_needs_its_information_characteristic() {
        let [_, keys, stream] = owon(CharPropFlags::READ);
        assert_eq!(choose([keys.clone(), stream.clone()]), Err("write"));

        let [unreadable, ..] = owon(CharPropFlags::WRITE);
        assert_eq!(choose([unreadable, keys, stream]), Err("write"));
    }

    /// Key presses go unacknowledged where FFF3 lists that, as a B41T+ acts
    /// on, and acknowledged where it lists only that.
    #[test]
    fn owon_keys_go_unacknowledged_where_fff3_takes_it() {
        let [_, keys, _] = owon(CharPropFlags::READ);
        assert_eq!(write_type(&OWON, &keys), WriteType::WithoutResponse);

        let acknowledged = characteristic(OWON.service, OWON.write, CharPropFlags::WRITE);
        assert_eq!(write_type(&OWON, &acknowledged), WriteType::WithResponse);
    }

    /// An over-MTU write is rejected by the peer, so the chunk size has to
    /// follow whatever the platform negotiated.
    #[test]
    fn writes_are_chunked_to_the_negotiated_mtu() {
        assert_eq!(write_chunk_size(23), 20);
        assert_eq!(write_chunk_size(247), 244);
        // Not negotiated yet, or a value too small to carry a write header.
        assert_eq!(write_chunk_size(0), DEFAULT_WRITE_CHUNK);
        assert_eq!(write_chunk_size(3), DEFAULT_WRITE_CHUNK);

        let frame = [0xABu8; 45];
        let chunks: Vec<usize> = frame
            .chunks(write_chunk_size(23))
            .map(<[u8]>::len)
            .collect();
        assert_eq!(chunks, vec![20, 20, 5]);
    }
}
