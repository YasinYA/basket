use basket::ui::format_relative_time_at;

#[test]
fn format_relative_time_ranges() {
    assert_eq!(format_relative_time_at(1000, 1000), "just now");
    assert_eq!(format_relative_time_at(940, 1000), "1m ago");
    assert_eq!(format_relative_time_at(3600, 7200), "1h ago");
    assert_eq!(format_relative_time_at(86_400, 172_800), "1d ago");
    assert_eq!(format_relative_time_at(604_800, 1_209_600), "1w ago");
}
