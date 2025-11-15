use blp::{AnyImage, AnyImageData};
use std::fs::read;

// PSD tests require a fixture `tests/fixtures/sample.psd` which isn't included
// in repository for size/licensing reasons. This test is ignored by default
// and will only be executed when the fixture is present and `--ignored` is passed.
#[test]
#[ignore]
fn test_psd_detect_and_frames() {
    let path = "tests/fixtures/sample.psd";
    let buf = read(path).expect("Please add tests/fixtures/sample.psd to run PSD tests");

    // Detection must succeed
    let any = AnyImage::from_buffer(&buf).expect("AnyImage should parse PSD");
    match &any.data {
        AnyImageData::Psd(_) => {},
        other => panic!("Expected PSD variant, got {:?}", other),
    }

    // PSD is single-frame in our model
    assert_eq!(any.frames.len(), 1);
}
