use super::*;

#[test]
fn reference_conversion_navigation_skips_rvc_only_fields() {
    assert_eq!(
        next_convert_field(ConvertField::Target, false),
        ConvertField::Consent
    );
    assert_eq!(
        previous_convert_field(ConvertField::Consent, false),
        ConvertField::Target
    );
    assert_eq!(
        next_convert_field(ConvertField::Target, true),
        ConvertField::F0Method
    );
}
