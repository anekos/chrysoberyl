
use std::env::home_dir;

use shorter::*;



macro_rules! assert_short {
    ($max:expr, $original:expr, $expect:expr) => {
        {
            let original = Path::new(&$original).to_path_buf();
            let shorten_path = shorten(&original, $max);
            assert_eq!(
                shorten_path,
                $expect.to_owned());
        }
    }

}

#[test]
fn test_shorten() {
    let home = path_to_string(&home_dir().unwrap());

    assert_short!(10000, format!("{}/hoge.jpg", home), "~/hoge.jpg".to_owned());

    //                 1234567890123456789    1234567890123456789
    assert_short!(30, "/hoge/moge/foge.jpg", "/hoge/moge/foge.jpg");
    //                 1234567890123456789    1234567890123
    assert_short!(15, "/hoge/moge/foge.jpg", "moge/foge.jpg");
    //                 1234567890123456789    12345678
    assert_short!(10, "/hoge/moge/foge.jpg", "foge.jpg");
    //                1234567890123456789    12345678
    assert_short!(1, "/hoge/moge/foge.jpg", "foge.jpg");
}
