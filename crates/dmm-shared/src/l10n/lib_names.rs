//! The library's enum names in the user's language, by variant: an
//! exhaustive `match` per enum, so a variant added in dmm-lib fails to
//! compile here until it has a message in `i18n/<locale>/lib-names.ftl`.
//! A test checks each en-US name against the library's own English.
//!
//! `Stability` is translated with the help text that tags device lists
//! ([`crate::help::Help::stability`]); `Setting::name` and `MockMode` labels
//! are typed by the user and stay English.

use super::Localizer;
use crate::tr;
use dmm_lib::flags::Flag;
use dmm_lib::measurement::MainLabel;
use dmm_lib::protocol::registry::Brand;
use dmm_lib::transport::Link;
use std::borrow::Cow;

impl Localizer {
    /// A status flag's label ([`Flag::label`]); `None` for the flag shown
    /// by no label.
    pub fn flag(&self, flag: Flag) -> Option<Cow<'_, str>> {
        Some(match flag {
            Flag::Hold => tr!(self, "flag-hold"),
            Flag::Rel => tr!(self, "flag-rel"),
            Flag::AutoRange => tr!(self, "flag-auto-range"),
            Flag::Min => tr!(self, "flag-min"),
            Flag::Max => tr!(self, "flag-max"),
            Flag::Avg => tr!(self, "flag-avg"),
            Flag::LowBattery => tr!(self, "flag-low-battery"),
            Flag::HvWarning => tr!(self, "flag-hv-warning"),
            Flag::PeakMax => tr!(self, "flag-peak-max"),
            Flag::PeakMin => tr!(self, "flag-peak-min"),
            Flag::LeadError => tr!(self, "flag-lead-error"),
            Flag::Comp => tr!(self, "flag-comp"),
            Flag::Record => tr!(self, "flag-record"),
            Flag::LoZ => tr!(self, "flag-loz"),
            Flag::Void => tr!(self, "flag-void"),
            Flag::Dc => return None,
        })
    }

    /// A link's full name ([`Link::full_name`]).
    pub fn link_name(&self, link: Link, built_in_radio: bool) -> Cow<'_, str> {
        match link {
            Link::UsbCable => tr!(self, "link-name-usb-cable"),
            Link::Bluetooth if built_in_radio => tr!(self, "link-name-bluetooth-link"),
            Link::Bluetooth => tr!(self, "link-name-bluetooth-adapter"),
        }
    }

    /// A main reading's name ([`MainLabel::as_str`]).
    pub fn main_label(&self, label: MainLabel) -> Cow<'_, str> {
        match label {
            MainLabel::Dc => tr!(self, "main-label-dc"),
            MainLabel::T1 => tr!(self, "main-label-t1"),
            MainLabel::T2 => tr!(self, "main-label-t2"),
            MainLabel::Relative => tr!(self, "main-label-relative"),
            MainLabel::PeakMax => tr!(self, "main-label-peak-max"),
        }
    }

    /// The sub-value a scaled reading keeps the meter's own reading under
    /// ([`dmm_lib::transform::RAW_LABEL`]); [`Localizer::label`] gives it for that sub-value.
    pub(crate) fn raw_label(&self) -> Cow<'_, str> {
        tr!(self, "raw-label")
    }

    /// A brand as the device lists head it ([`Brand::name`]): the real ones
    /// as they are, the simulated meters' heading translated.
    pub fn brand(&self, brand: Brand) -> Cow<'_, str> {
        match brand {
            Brand::Simulated => tr!(self, "brand-simulated"),
            // Proper nouns, listed so a new brand has to choose.
            Brand::Aneng
            | Brand::Brymen
            | Brand::Bside
            | Brand::Eevblog
            | Brand::Minipa
            | Brand::Owon
            | Brand::UniT
            | Brand::Voltcraft
            | Brand::Zotek => Cow::Borrowed(brand.name()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l10n::LIB_RESOURCES;
    use dmm_lib::protocol::Stability;

    #[test]
    fn english_names_are_the_librarys() {
        let l = Localizer::en_us(LIB_RESOURCES);
        for flag in Flag::ALL {
            assert_eq!(l.flag(flag).as_deref(), flag.label(), "{flag:?}");
        }
        for link in [Link::UsbCable, Link::Bluetooth] {
            for radio in [false, true] {
                assert_eq!(l.link_name(link, radio), link.full_name(radio));
            }
        }
        for label in [
            MainLabel::Dc,
            MainLabel::T1,
            MainLabel::T2,
            MainLabel::Relative,
            MainLabel::PeakMax,
        ] {
            assert_eq!(l.main_label(label), label.as_str());
        }
        for brand in Brand::ALL {
            assert_eq!(l.brand(brand), brand.name());
        }
        assert_eq!(l.raw_label(), dmm_lib::transform::RAW_LABEL);
        let help = crate::help::english();
        for s in [
            Stability::Verified,
            Stability::PartlyVerified,
            Stability::Experimental,
        ] {
            assert_eq!(help.stability(s), s.label());
        }
    }
}
