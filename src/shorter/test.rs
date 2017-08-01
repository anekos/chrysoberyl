
use std::env::home_dir;

use shorter::*;



macro_rules! assert_short_path {
    ($max:expr, $original:expr, $expect:expr) => {
        {
            let original = Path::new(&$original).to_path_buf();
            let actual = shorten_path(&original, $max);
            assert_eq!(
                actual,
                $expect.to_owned());
        }
    }

}


macro_rules! assert_short_url {
    ($max:expr, $original:expr, $expect:expr) => {
        {
            let original = Url::parse(&$original).unwrap();
            let actual = shorten_url(original, $max);
            assert_eq!(
                actual,
                $expect.to_owned());
        }
    }

}

#[test]
fn test_shorten_path() {
    let home = path_to_string(&home_dir().unwrap());

    assert_short_path!(10000, format!("{}/hoge.jpg", home), "~/hoge.jpg".to_owned());

    //                      1234567890123456789    1234567890123456789
    assert_short_path!(30, "/hoge/moge/foge.jpg", "/hoge/moge/foge.jpg");
    //                      1234567890123456789    1234567890123
    assert_short_path!(15, "/hoge/moge/foge.jpg", "moge/foge.jpg");
    //                      1234567890123456789    12345678
    assert_short_path!(10, "/hoge/moge/foge.jpg", "foge.jpg");
    //                     1234567890123456789    12345678
    assert_short_path!(1, "/hoge/moge/foge.jpg", "foge.jpg");
}

#[test]
fn test_shorten_url() {
    assert_short_url!(100, "http://example.com/foo/bar.png", "example/foo/bar.png");
    assert_short_url!(100, "http://example.com/foo/bar.png?query", "example/foo/bar.png");
    assert_short_url!(100, "http://example.com/foo/bar.png?query#fragment", "example/foo/bar.png");
}
