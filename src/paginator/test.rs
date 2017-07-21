
use paginator::*;
use paginator::values::*;



macro_rules! assert_pg {
    ($function:ident,
     [fly_leaves: $t_fly_leaves:expr, len: $t_len:expr, level: $t_level:expr, sight_size: $t_sight_size:expr],
     [$($args:expr),*],
     $updated:expr,
     [fly_leaves: $e_fly_leaves:expr, level: $e_level:expr]) => {
        {
            let mut target = Paginator {
                fly_leaves: FlyLeaves($t_fly_leaves),
                len: $t_len,
                level: $t_level.map(Level),
                sight_size: SightSize($t_sight_size),
            };
            let expected = Paginator {
                fly_leaves: FlyLeaves($e_fly_leaves),
                len: $t_len,
                level: $e_level.map(Level),
                sight_size: SightSize($t_sight_size),
            };
            let updated = target.$function($($args),*);

            match (&target, &expected) {
                (target, expected) => {
                    if *target != *expected {
                        panic!("assertion failed:\nactual:   {:?}\nexpected: {:?}", target, expected);
                    }
                }
            }

            assert_eq!(target, expected);
            assert_eq!(updated, $updated);
        }

    }
}

macro_rules! assert_pg_mv {
    ($function:ident,
     [fly_leaves: $t_fly_leaves:expr, len: $t_len:expr, level: $t_level:expr, sight_size: $t_sight_size:expr],
     [count: $count:expr, ignore_sight: $ignore_sight:expr, wrap: $wrap:expr],
     $updated:expr,
     [fly_leaves: $e_fly_leaves:expr, level: $e_level:expr]) => {
        {
            let paging = Paging {
                count: $count,
                ignore_sight: $ignore_sight,
                wrap: $wrap,
            };
            assert_pg!(
                $function,
                [fly_leaves: $t_fly_leaves, len: $t_len, level: $t_level, sight_size: $t_sight_size],
                [paging],
                $updated,
                [fly_leaves: $e_fly_leaves, level: $e_level]);
        }
    }
}


#[test]
fn test_index_with_sight_size() {
    assert_eq!(
        Index(0).with_sight_size(SightSize(1)),
        FlyLeaves(0));

    assert_eq!(
        Index(2).with_sight_size(SightSize(1)),
        FlyLeaves(0));

    assert_eq!(
        Index(0).with_sight_size(SightSize(4)),
        FlyLeaves(0));

    assert_eq!(
        Index(1).with_sight_size(SightSize(4)),
        FlyLeaves(3));

    assert_eq!(
        Index(2).with_sight_size(SightSize(4)),
        FlyLeaves(2));

    assert_eq!(
        Index(3).with_sight_size(SightSize(4)),
        FlyLeaves(1));

    assert_eq!(
        Index(4).with_sight_size(SightSize(4)),
        FlyLeaves(0));

    assert_eq!(
        Index(5).with_sight_size(SightSize(4)),
        FlyLeaves(3));

    assert_eq!(
        Index(6).with_sight_size(SightSize(4)),
        FlyLeaves(2));
}

#[test]
fn test_increase_level() {
    /**
     *  00      00
     *  01     <01>
     *  02      02
     *  03  +1  03
     */
    assert_pg!(
        increase_level,
        [fly_leaves: 0, len: 4, level: None, sight_size: 1],
        [1, false],
        true,
        [fly_leaves: 0, level: Some(1)]);

    /**
     * <00>01 02 03       00 01 02 03
     *  04 05 06 07      <04>05 06 07
     *  08 09 10 11       08 09 10 11
     *  12 13 14     +1   12 13 14
     */
    assert_pg!(
        increase_level,
        [fly_leaves: 0, len: 15, level: Some(0), sight_size: 4],
        [1, false],
        true,
        [fly_leaves: 0, level: Some(1)]);

    /**
     *  00 01 02 03       00 01 02 03
     * <04>05 06 07       04 05 06 07
     *  08 09 10 11      <08>09 10 11
     *  12 13 14     +1   12 13 14
     */
    assert_pg!(
        increase_level,
        [fly_leaves: 0, len: 15, level: Some(1), sight_size: 4],
        [1, false],
        true,
        [fly_leaves: 0, level: Some(2)]);

    /**
     * <00>     00
     *  01      01
     *  02      02
     *  03  +3 <03>
     */
    assert_pg!(
        increase_level,
        [fly_leaves: 0, len: 4, level: Some(0), sight_size: 1],
        [3, false],
        true,
        [fly_leaves: 0, level: Some(3)]);

    /**
     * <00>01 02 03       00 01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09 10 11       08 09 10 11
     *  12 13 14     +3  <12>13 14
     */
    assert_pg!(
        increase_level,
        [fly_leaves: 0, len: 15, level: Some(0), sight_size: 4],
        [3, false],
        true,
        [fly_leaves: 0, level: Some(3)]);

    /**
     * <00>     00
     *  01      01
     *  02      02
     *  03  +8 <03>
     */
    assert_pg!(
        increase_level,
        [fly_leaves: 0, len: 4, level: Some(0), sight_size: 1],
        [8, false],
        true,
        [fly_leaves: 0, level: Some(3)]);

    /**
     * <00>01 02 03       00 01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09 10 11       08 09 10 11
     *  12 13 14     +8  <12>13 14
     */
    assert_pg!(
        increase_level,
        [fly_leaves: 0, len: 15, level: Some(0), sight_size: 4],
        [3, false],
        true,
        [fly_leaves: 0, level: Some(3)]);

    /**
     * <XX>XX XX 00       XX XX XX 00
     *  01 02 03 04       01 02 03 04
     *  05 06 07 08       05 06 07 08
     *  09 10 11     +3  <09>10 11
     */
    assert_pg!(
        increase_level,
        [fly_leaves: 3, len: 12, level: Some(0), sight_size: 4],
        [3, false],
        true,
        [fly_leaves: 3, level: Some(3)]);

    /**
     * wrap
     *
     * <00>     00
     *  01      01
     *  02      02
     *  03  +3 <03>
     */
    assert_pg!(
        increase_level,
        [fly_leaves: 0, len: 4, level: Some(0), sight_size: 1],
        [3, true],
        true,
        [fly_leaves: 0, level: Some(3)]);

    /**
     * wrap
     *
     * <00>01 02 03       00 01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09 10 11       08 09 10 11
     *  12 13 14     +3  <12>13 14
     */
    assert_pg!(
        increase_level,
        [fly_leaves: 0, len: 15, level: Some(0), sight_size: 4],
        [3, true],
        true,
        [fly_leaves: 0, level: Some(3)]);

    /**
     * wrap
     *
     * <00>    <00>
     *  01      01
     *  02      02
     *  03  +4  03
     */
    assert_pg!(
        increase_level,
        [fly_leaves: 0, len: 4, level: Some(0), sight_size: 1],
        [4, true],
        false,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * wrap
     *
     * <00>01 02 03      <00>01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09 10 11       08 09 10 11
     *  12 13 14     +4   12 13 14
     */
    assert_pg!(
        increase_level,
        [fly_leaves: 0, len: 15, level: Some(0), sight_size: 4],
        [4, true],
        false,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * wrap
     *
     * <00>01 02 03       00 01 02 03
     *  04 05 06 07      <04>05 06 07
     *  08 09 10 11       08 09 10 11
     *  12 13 14     +5   12 13 14
     */
    assert_pg!(
        increase_level,
        [fly_leaves: 0, len: 15, level: Some(0), sight_size: 4],
        [5, true],
        true,
        [fly_leaves: 0, level: Some(1)]);

    /**
     * wrap
     *
     * <XX>XX XX 00      <XX>XX XX 00
     *  01 02 03 04       01 02 03 04
     *  05 06 07 08       05 06 07 08
     *  09 10 11     +4   09 10 11
     */
    assert_pg!(
        increase_level,
        [fly_leaves: 3, len: 12, level: Some(0), sight_size: 4],
        [4, true],
        false,
        [fly_leaves: 3, level: Some(0)]);
}

#[test]
fn test_decrease_level() {
    /**
     *  00     <00>
     *  01      01 
     *  02      02
     *  03  -1  03
     */
    assert_pg!(
        decrease_level,
        [fly_leaves: 0, len: 4, level: None, sight_size: 1],
        [1, false],
        true,
        [fly_leaves: 0, level: Some(0)]);

    /**
     *  00 01 02 03      <00>01 02 03
     * <04>05 06 07       04 05 06 07
     *  08 09 10 11       08 09 10 11
     *  12 13 14     -1   12 13 14
     */
    assert_pg!(
        decrease_level,
        [fly_leaves: 0, len: 15, level: Some(1), sight_size: 4],
        [1, false],
        true,
        [fly_leaves: 0, level: Some(0)]);

    /**
     *  00 01 02 03       00 01 02 03
     *  04 05 06 07      <04>05 06 07
     * <08>09 10 11       08 09 10 11
     *  12 13 14     +1   12 13 14
     */
    assert_pg!(
        decrease_level,
        [fly_leaves: 0, len: 15, level: Some(2), sight_size: 4],
        [1, false],
        true,
        [fly_leaves: 0, level: Some(1)]);

    /**
     *  00     <00>
     *  01      01
     *  02      02
     * <03> +3  03 
     */
    assert_pg!(
        decrease_level,
        [fly_leaves: 0, len: 4, level: Some(3), sight_size: 1],
        [3, true],
        true,
        [fly_leaves: 0, level: Some(0)]);

    /**
     *  00 01 02 03      <00>01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09 10 11       08 09 10 11
     * <12>13 14     -3   12 13 14
     */
    assert_pg!(
        decrease_level,
        [fly_leaves: 0, len: 15, level: Some(3), sight_size: 4],
        [3, false],
        true,
        [fly_leaves: 0, level: Some(0)]);

    /**
     *  00     <00>
     *  01      01
     *  02      02
     * <03> +8  03
     */
    assert_pg!(
        decrease_level,
        [fly_leaves: 0, len: 4, level: Some(3), sight_size: 1],
        [8, false],
        true,
        [fly_leaves: 0, level: Some(0)]);

    /**
     *  00 01 02 03      <00>01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09 10 11       08 09 10 11
     * <12>13 14     +8   12 13 14
     */
    assert_pg!(
        decrease_level,
        [fly_leaves: 0, len: 15, level: Some(3), sight_size: 4],
        [3, false],
        true,
        [fly_leaves: 0, level: Some(0)]);

    /**
     *  XX XX XX 00      <XX>XX XX 00
     *  01 02 03 04       01 02 03 04
     *  05 06 07 08       05 06 07 08
     * <09>10 11     +3   09 10 11
     */
    assert_pg!(
        decrease_level,
        [fly_leaves: 3, len: 12, level: Some(3), sight_size: 4],
        [3, false],
        true,
        [fly_leaves: 3, level: Some(0)]);

    /**
     * wrap
     *
     *  00     <00>
     *  01      01
     *  02      02
     * <03> +3  03 
     */
    assert_pg!(
        decrease_level,
        [fly_leaves: 0, len: 4, level: Some(3), sight_size: 1],
        [3, true],
        true,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * wrap
     *
     *  00 01 02 03      <00>01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09 10 11       08 09 10 11
     * <12>13 14     +3   12 13 14
     */
    assert_pg!(
        decrease_level,
        [fly_leaves: 0, len: 15, level: Some(3), sight_size: 4],
        [3, true],
        true,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * wrap
     *
     * <00>     00
     *  01     <01>
     *  02      02
     *  03  +3  03
     */
    assert_pg!(
        decrease_level,
        [fly_leaves: 0, len: 4, level: Some(0), sight_size: 1],
        [3, true],
        true,
        [fly_leaves: 0, level: Some(1)]);

    /**
     * wrap
     *
     * <00>    <00>
     *  01      01
     *  02      02
     *  03  +4  03
     */
    assert_pg!(
        decrease_level,
        [fly_leaves: 0, len: 4, level: Some(0), sight_size: 1],
        [4, true],
        false,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * wrap
     *
     * <00>01 02 03      <00>01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09 10 11       08 09 10 11
     *  12 13 14     +4   12 13 14
     */
    assert_pg!(
        decrease_level,
        [fly_leaves: 0, len: 15, level: Some(0), sight_size: 4],
        [4, true],
        false,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * wrap
     *
     * <00>01 02 03       00 01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09 10 11       08 09 10 11
     *  12 13 14     +5  <12>13 14
     */
    assert_pg!(
        decrease_level,
        [fly_leaves: 0, len: 15, level: Some(0), sight_size: 4],
        [5, true],
        true,
        [fly_leaves: 0, level: Some(3)]);

    /**
     * wrap
     *
     * <XX>XX XX 00      <XX>XX XX 00
     *  01 02 03 04       01 02 03 04
     *  05 06 07 08       05 06 07 08
     *  09 10 11     +4   09 10 11
     */
    assert_pg!(
        decrease_level,
        [fly_leaves: 3, len: 12, level: Some(0), sight_size: 4],
        [4, true],
        false,
        [fly_leaves: 3, level: Some(0)]);
}


#[test]
fn test_next() {
    /**
     * empty container
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 0, len: 0, level: None, sight_size: 1],
        [count: 1, ignore_sight: false, wrap: false],
        false,
        [fly_leaves: 0, level: None]);

    /**
     * empty container
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 0, len: 0, level: None, sight_size: 4],
        [count: 1, ignore_sight: false, wrap: false],
        false,
        [fly_leaves: 0, level: None]);

    /**
     * empty container
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 0, len: 0, level: None, sight_size: 4],
        [count: 1, ignore_sight: true, wrap: true],
        false,
        [fly_leaves: 0, level: None]);

    /**
     * <00>     00
     *  01     <01>
     *  02  +1  02
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 0, len: 3, level: Some(0), sight_size: 1],
        [count: 1, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 0, level: Some(1)]);

    /**
     * <00>     00
     *  01      01
     *  02      02
     *  03      03
     *  04     <04>
     *  05      05
     *  06  +1  06
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 0, len: 7, level: Some(0), sight_size: 1],
        [count: 4, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 0, level: Some(4)]);

    /**
     * <00>     00
     *  01      01
     *  02      02
     *  03      03
     *  04     <04>
     *  05      05
     *  06  +6  06
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 0, len: 7, level: Some(0), sight_size: 1],
        [count: 6, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 0, level: Some(6)]);

    /**
     * <00>     00
     *  01      01
     *  02      02
     *  03  +3 <03>
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 0, len: 4, level: Some(0), sight_size: 1],
        [count: 3, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 0, level: Some(3)]);

    /**
     * <00>     00
     *  01      01
     *  02      02
     *  03  +4 <03>
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 0, len: 4, level: Some(0), sight_size: 1],
        [count: 4, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 0, level: Some(3)]);

    /**
     * <00>     00
     *  01      01
     *  02      02
     *  03  +9 <03>
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 0, len: 4, level: Some(0), sight_size: 1],
        [count: 9, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 0, level: Some(3)]);

    /**
     *  XX XX 00 01      XX XX 00 01
     * <02>03 04 05      02 03 04 05
     *  06 07 08 09     <06>07 08 09
     *  10 11 12 13  +1  10 11 12 13
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 2, len: 14, level: Some(1), sight_size: 4],
        [count: 1, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 2, level: Some(2)]);

    /**
     *  XX XX 00 01      XX XX 00 01
     * <02>03 04 05      02 03 04 05
     *  06 07 08 09      06 07 08 09
     *  10 11 12 13  +2 <10>11 12 13
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 2, len: 14, level: Some(1), sight_size: 4],
        [count: 2, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 2, level: Some(3)]);

    /**
     *  XX XX 00 01      XX XX 00 01
     * <02>03 04 05      02 03 04 05
     *  06 07 08 09      06 07 08 09
     *  10 11 12 13  +3 <10>11 12 13
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 2, len: 14, level: Some(1), sight_size: 4],
        [count: 3, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 2, level: Some(3)]);

    /**
     * ignore_sight
     *
     *  XX XX 00 01      XX 00 01 02
     * <02>03 04 05      03 04 05 06
     *  06 07 08 09     <07>08 09 10
     *  10 11 12 13  +5  11 12 13
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 2, len: 14, level: Some(1), sight_size: 4],
        [count: 5, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 1, level: Some(2)]);

    /**
     * ignore_sight
     *
     *  XX XX 00 01       XX XX 00 01
     * <02>03 04 05       02 03 04 05
     *  06 07 08 09       06 07 08 09
     *  10 11 12 13  +8  <10>11 12 13
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 2, len: 14, level: Some(1), sight_size: 4],
        [count: 8, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 2, level: Some(3)]);

    /**
     * ignore_sight
     *
     *  XX XX 00 01 02       XX XX XX 00 01
     * <03>04 05 06 07       02 03 04 05 06
     *  08 09 10 11 12       07 08 09 10 11 
     *  13 14 15 16 17      <12>13 14 15 16
     *  18              +9   17 18
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 2, len: 19, level: Some(1), sight_size: 5],
        [count: 9, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 3, level: Some(3)]);

    /**
     * ignore_sight
     *
     *  XX XX 00 01 02        XX XX XX 00 01
     * <03>04 05 06 07        02 03 04 05 06
     *  08 09 10 11 12        07 08 09 10 11 
     *  13 14 15 16 17        12 13 14 15 16
     *  18              +14  <17>18
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 2, len: 19, level: Some(1), sight_size: 5],
        [count: 14, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 3, level: Some(4)]);

    /**
     * ignore_sight
     *
     *  XX XX 00 01        XX XX XX 00
     * <02>03 04 05        01 02 03 04
     *  06 07 08 09        05 06 07 08
     *  10 11 12 13        09 10 11 12
     *               +11  <13>
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 2, len: 14, level: Some(1), sight_size: 4],
        [count: 11, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 3, level: Some(4)]);

    /**
     * ignore_sight
     *
     *  XX XX 00 01        XX XX XX 00
     * <02>03 04 05        01 02 03 04
     *  06 07 08 09        05 06 07 08
     *  10 11 12 13        09 10 11 12
     *  14           +11  <13>14
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 2, len: 14, level: Some(1), sight_size: 4],
        [count: 11, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 3, level: Some(4)]);

    /**
     * ignore_sight
     *
     *  XX XX 00 01        XX XX XX 00
     * <02>03 04 05        01 02 03 04
     *  06 07 08 09        05 06 07 08
     *  10 11 12 13        09 10 11 12
     *               +20  <13>
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 2, len: 14, level: Some(1), sight_size: 4],
        [count: 20, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 3, level: Some(4)]);

    /**
     * ignore_sight
     *
     * <00>01 02 03       XX XX XX 00
     *  04 05 06 07      <01>02 03 04
     *               +1   05 06 07
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 0, len: 8, level: Some(0), sight_size: 4],
        [count: 1, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 3, level: Some(1)]);

    /**
     * ignore_sight
     *
     *                    XX XX XX 00
     *  00 01 02 03       01 02 03 04
     * <04>05 06 07  +1  <05>06 07
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 0, len: 10, level: Some(1), sight_size: 4],
        [count: 1, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 3, level: Some(2)]);

    /**
     * ignore_sight
     *
     *  XX XX 00 01       XX 00 01 02
     * <02>03 04 05      <03>04 05 06
     *  06 07 08 09  +1   07 08 09
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 2, len: 10, level: Some(1), sight_size: 4],
        [count: 1, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 1, level: Some(1)]);

    /**
     * ignore_sight
     *
     *  XX XX XX 00       XX XX 00 01
     *  01 02 03 04       02 03 04 05
     * <05>06 07 08  +1  <06>07 08
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 3, len: 9, level: Some(2), sight_size: 4],
        [count: 1, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 2, level: Some(2)]);

    /**
     * ignore_sight
     *
     *  XX XX XX 00       00 01 02 03
     *  01 02 03 04       04 05 06 07
     * <05>06 07 08  +3  <08>
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 3, len: 9, level: Some(2), sight_size: 4],
        [count: 3, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 0, level: Some(2)]);

    /**
     * ignore_sight
     *
     *  XX XX XX 00       XX XX XX 00
     *  01 02 03 04       01 02 03 04
     * <05>06 07 08  +4   05 06 07 08
     *  09 10            <09>10 11
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 3, len: 12, level: Some(2), sight_size: 4],
        [count: 4, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 3, level: Some(3)]);

    /**
     * ignore_sight
     *
     *  XX XX XX 00       XX XX XX 00
     *  01 02 03 04       01 02 03 04
     * <05>          +1  <05>
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 3, len: 6, level: Some(2), sight_size: 4],
        [count: 1, ignore_sight: true, wrap: false],
        false,
        [fly_leaves: 3, level: Some(2)]);

    /**
     * wrap
     *
     *  <00>       00
     *   01       <01>
     *   02   +1   02
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 1],
        [count: 10, ignore_sight: false, wrap: true],
        false,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * wrap
     *
     *   00       <00>
     *   01        01
     *  <02>  +1   02
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 0, len: 3, level: Some(2), sight_size: 1],
        [count: 1, ignore_sight: false, wrap: true],
        true,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * wrap
     *
     *  00 01 02 03      <00>01 02 03
     *  04 05 06 07       04 05 06 07
     * <08>09 10 11  +1   08 09 10 11
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 0, len: 12, level: Some(2), sight_size: 4],
        [count: 1, ignore_sight: false, wrap: true],
        true,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * wrap
     *
     *  00 01 02 03       00 01 02 03
     *  04 05 06 07      <04>05 06 07
     * <08>09 10 11  +2   08 09 10 11
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 0, len: 12, level: Some(2), sight_size: 4],
        [count: 2, ignore_sight: false, wrap: true],
        true,
        [fly_leaves: 0, level: Some(1)]);

    /**
     * wrap
     *
     *  00 01 02 03      <00>01 02 03
     *  04 05 06 07       04 05 06 07
     * <08>09        +1   08 09
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 0, len: 10, level: Some(2), sight_size: 4],
        [count: 1, ignore_sight: false, wrap: true],
        true,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * initial
     *
     *   00        00
     *   01       <01>
     *   02   +1   02
     */
    assert_pg_mv!(
        next,
        [fly_leaves: 0, len: 3, level: None, sight_size: 1],
        [count: 1, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 0, level: Some(1)]);
}

#[test]
fn test_previous() {
    /**
     *  00        00
     *  01       <01>
     * <02>  -1   02
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 0, len: 3, level: Some(2), sight_size: 1],
        [count: 1, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 0, level: Some(1)]);

    /**
     *  00       <00>
     *  01        01
     * <02>  -2   02
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 0, len: 3, level: Some(2), sight_size: 1],
        [count: 2, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 0, level: Some(0)]);

    /**
     *  00       <00>
     *  01        01
     *  02        02
     * <03>  -4   03
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 0, len: 4, level: Some(3), sight_size: 1],
        [count: 4, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 0, level: Some(0)]);

    /**
     *  00 01 02 03       <00>01 02 03
     *  04 05 06 07        04 05 06 07
     * <08>09        -3    08 09
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 0, len: 10, level: Some(2), sight_size: 1],
        [count: 3, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 0, level: Some(0)]);

    /**
     *  00 01 02 03       00 01 02 03
     *  04 05 06 07      <04>05 06 07
     * <08>09        -1   08 09
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 0, len: 10, level: Some(2), sight_size: 4],
        [count: 1, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 0, level: Some(1)]);

    /**
     *  00 01 02 03      <00>01 02 03
     *  04 05 06 07       04 05 06 07
     * <08>09        -2   08 09
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 0, len: 10, level: Some(2), sight_size: 4],
        [count: 2, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 0, level: Some(0)]);

    /**
     *  00 01 02 03       00 01 02 03
     *  04 05 06 07      <04>05 06 07
     * <08>09        -3   08 09
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 0, len: 10, level: Some(2), sight_size: 4],
        [count: 3, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * ignore_sight
     *
     *  00 01 02 03       XX XX 00 01
     *  04 05 06 07       02 03 04 05
     * <08>09        -2  <06>07 08 09
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 0, len: 10, level: Some(2), sight_size: 4],
        [count: 2, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 2, level: Some(2)]);

    /**
     * ignore_sight
     *
     *  00 01 02 03       XX XX 00 01
     *  04 05 06 07       02 03 04 05
     * <08>09 10         <06>07 08 09
     *               -2   10
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 0, len: 11, level: Some(2), sight_size: 4],
        [count: 2, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 2, level: Some(2)]);

    /**
     * ignore_sight
     *
     *  00 01 02 03       XX XX XX 00
     *  04 05 06 07       01 02 03 04
     * <08>09 10         <05>06 07 08
     *               -3   09 10
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 0, len: 11, level: Some(2), sight_size: 4],
        [count: 3, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 3, level: Some(2)]);

    /**
     * ignore_sight
     *
     *  00 01 02 03       00 01 02 03
     *  04 05 06 07      <04>05 06 07
     * <08>09 10     -4   08 09 10
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 0, len: 11, level: Some(2), sight_size: 4],
        [count: 4, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 0, level: Some(1)]);

    /**
     * wrap
     *
     *  00 01 02 03       00 01 02 03
     *  04 05 06 07       04 05 06 07
     * <08>09 10 11  -3  <08>09 10 11
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 0, len: 12, level: Some(2), sight_size: 4],
        [count: 3, ignore_sight: false, wrap: true],
        false,
        [fly_leaves: 0, level: Some(2)]);

    /**
     * wrap
     *
     *  00 01 02 03       00 01 02 03
     *  04 05 06 07       04 05 06 07
     * <08>09        -3  <08>09
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 0, len: 10, level: Some(2), sight_size: 4],
        [count: 3, ignore_sight: false, wrap: true],
        false,
        [fly_leaves: 0, level: Some(2)]);

    /**
     * wrap
     *
     *  00 01 02 03       00 01 02 03
     *  04 05 06 07       04 05 06 07
     * <08>09        -3  <08>09
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 0, len: 10, level: Some(2), sight_size: 4],
        [count: 3, ignore_sight: false, wrap: true],
        false,
        [fly_leaves: 0, level: Some(2)]);

    /**
     * wrap
     *
     *  00 01 02 03       00 01 02 03
     *  04 05 06 07      <04>05 06 07
     * <08>09        -4   08 09
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 0, len: 10, level: Some(2), sight_size: 4],
        [count: 4, ignore_sight: false, wrap: true],
        true,
        [fly_leaves: 0, level: Some(1)]);

    /**
     * <00>01 02 03      <00>01 02 03
     *  04 05 06 07  -1   04 05 06 07
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 0, len: 8, level: Some(0), sight_size: 4],
        [count: 1, ignore_sight: false, wrap: false],
        false,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * <00>01 02 03      <00>01 02 03
     *  04 05 06 07  -4   04 05 06 07
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 0, len: 8, level: Some(0), sight_size: 4],
        [count: 4, ignore_sight: false, wrap: false],
        false,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * <XX>XX XX 00  -1  <XX>XX XX 00
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 0, len: 1, level: Some(0), sight_size: 4],
        [count: 1, ignore_sight: false, wrap: false],
        false,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * ignore_sight
     *
     * <XX>XX XX 00  -1  <XX>XX XX 00
     *  01 02 03          01 02 03
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 3, len: 4, level: Some(0), sight_size: 4],
        [count: 1, ignore_sight: true, wrap: false],
        false,
        [fly_leaves: 3, level: Some(0)]);

    /**
     *  XX XX XX 00       00 01 02 03
     *  01 02 03 04      <04>05 06 07
     * <05>06 07 08  -1   08
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 3, len: 9, level: Some(2), sight_size: 4],
        [count: 1, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 0, level: Some(1)]);

    /**
     *  XX XX XX 00      <00>01 02 03
     * <01>02 03 04       04 05 06 07
     *  05 06 07 09  -1   08 09
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 3, len: 10, level: Some(1), sight_size: 4],
        [count: 1, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * wrap
     *
     *  XX XX XX 00       XX XX XX 00
     * <01>02 03 04       01 02 03 04
     *  05 06 07 08  -8  <05>06 07 08
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 3, len: 9, level: Some(1), sight_size: 4],
        [count: 8, ignore_sight: false, wrap: true],
        true,
        [fly_leaves: 3, level: Some(2)]);

    /**
     * wrap
     *
     *  XX XX XX 00       XX XX XX 00
     * <01>02 03 04      <01>02 03 04
     *  05 06 07 08       05 06 07 08
     *  09 10 11 12  -8   09 10 11 12
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 3, len: 13, level: Some(1), sight_size: 4],
        [count: 8, ignore_sight: false, wrap: true],
        false,
        [fly_leaves: 3, level: Some(1)]);

    /**
     * wrap
     *
     *  XX XX XX 00       XX XX XX 00
     * <01>02 03 04       01 02 03 04
     *  05 06 07 08      <05>06 07 08
     *  09 10 11 12  -7   09 10 11 12
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 3, len: 13, level: Some(1), sight_size: 4],
        [count: 7, ignore_sight: false, wrap: true],
        true,
        [fly_leaves: 3, level: Some(2)]);

    /**
     * wrap
     *
     *  XX XX XX 00      <XX>XX XX 00
     * <01>02 03 04       01 02 03 04
     *  05 06 07 08       05 06 07 08
     *  09 10 11 12  -9   09 10 11 12
     */
    assert_pg_mv!(
        previous,
        [fly_leaves: 3, len: 13, level: Some(1), sight_size: 4],
        [count: 9, ignore_sight: false, wrap: true],
        true,
        [fly_leaves: 3, level: Some(0)]);
}

#[test]
fn test_first() {
    /**
     * <00>01 02 03      <00>01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09        =1   08 09
     */
    assert_pg_mv!(
        first,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 1, ignore_sight: false, wrap: false],
        false,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * <00>01 02 03       00 01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09        =2  <08>09
     */
    assert_pg_mv!(
        first,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 2, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 0, level: Some(1)]);

    /**
     * <00>01 02 03       00 01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09        =3  <08>09
     */
    assert_pg_mv!(
        first,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 3, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 0, level: Some(2)]);

    /**
     *  00 01 02 03       00 01 02 03
     * <04>05 06 07      <04>05 06 07
     *  08 09        =1   08 09
     */
    assert_pg_mv!(
        first,
        [fly_leaves: 0, len: 10, level: Some(1), sight_size: 4],
        [count: 2, ignore_sight: false, wrap: false],
        false,
        [fly_leaves: 0, level: Some(1)]);

    /**
     * wrap
     *
     * <00>01 02 03       00 01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09        =3  <08>09
     */
    assert_pg_mv!(
        first,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 3, ignore_sight: false, wrap: true],
        true,
        [fly_leaves: 0, level: Some(2)]);

    /**
     * wrap
     *
     * <00>01 02 03      <00>01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09        =4   08 09
     */
    assert_pg_mv!(
        first,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 4, ignore_sight: false, wrap: true],
        false,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * wrap
     *
     * <00>01 02 03       00 01 02 03
     *  04 05 06 07      <04>05 06 07
     *  08 09        =5   08 09
     */
    assert_pg_mv!(
        first,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 5, ignore_sight: false, wrap: true],
        true,
        [fly_leaves: 0, level: Some(1)]);

    /**
     * ignore_sight
     *
     * <00>01 02 03      <00>01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09        =1   08 09
     */
    assert_pg_mv!(
        first,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 1, ignore_sight: true, wrap: false],
        false,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * ignore_sight
     *
     * <00>01 02 03       XX XX XX 00
     *  04 05 06 07      <01>02 03 04
     *  08 09             05 06 07 08
     *               =2   09
     */
    assert_pg_mv!(
        first,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 2, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 3, level: Some(1)]);

    /**
     * ignore_sight
     *
     * <00>01 02 03       XX XX 00 01
     *  04 05 06 07      <02>03 04 05
     *  08 09             06 07 08 09
     *               =3   09
     */
    assert_pg_mv!(
        first,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 3, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 2, level: Some(1)]);

    /**
     * ignore_sight
     *
     * <00>01 02 03       XX 00 01 02
     *  04 05 06 07       03 04 05 06
     *  08 09        =8  <07>08 09
     */
    assert_pg_mv!(
        first,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 8, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 1, level: Some(2)]);

    /**
     * ignore_sight
     *
     *                     XX XX XX 00
     * <00>01 02 03        01 02 03 04
     *  04 05 06 07        05 06 07 08
     *  08 09        =20  <09>
     */
    assert_pg_mv!(
        first,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 20, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 3, level: Some(3)]);
}

#[test]
fn test_last() {
    /**
     * <00>01 02 03       00 01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09        =1  <08>09
     */
    assert_pg_mv!(
        last,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 1, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 0, level: Some(2)]);

    /**
     * <00>01 02 03       00 01 02 03
     *  04 05 06 07      <04>05 06 07
     *  08 09        =2   08 09
     */
    assert_pg_mv!(
        last,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 2, ignore_sight: false, wrap: false],
        true,
        [fly_leaves: 0, level: Some(1)]);

    /**
     * <00>01 02 03      <00>01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09        =3   08 09
     */
    assert_pg_mv!(
        last,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 3, ignore_sight: false, wrap: false],
        false,
        [fly_leaves: 0, level: Some(0)]);

    /**
     *  00 01 02 03       00 01 02 03
     * <04>05 06 07      <04>05 06 07
     *  08 09        =1   08 09
     */
    assert_pg_mv!(
        last,
        [fly_leaves: 0, len: 10, level: Some(1), sight_size: 4],
        [count: 2, ignore_sight: false, wrap: false],
        false,
        [fly_leaves: 0, level: Some(1)]);

    /**
     * wrap
     *
     * <00>01 02 03      <00>01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09        =3   08 09
     */
    assert_pg_mv!(
        last,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 3, ignore_sight: false, wrap: true],
        false,
        [fly_leaves: 0, level: Some(0)]);

    /**
     * wrap
     *
     * <00>01 02 03       00 01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09        =4  <08>09
     */
    assert_pg_mv!(
        last,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 4, ignore_sight: false, wrap: true],
        true,
        [fly_leaves: 0, level: Some(2)]);

    /**
     * wrap
     *
     * <00>01 02 03       00 01 02 03
     *  04 05 06 07      <04>05 06 07
     *  08 09        =5   08 09
     */
    assert_pg_mv!(
        last,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 5, ignore_sight: false, wrap: true],
        true,
        [fly_leaves: 0, level: Some(1)]);

    /**
     * ignore_sight
     *
     *                    XX XX XX 00
     * <00>01 02 03       01 02 03 04
     *  04 05 06 07       05 06 07 08
     *  08 09        =1  <09>
     */
    assert_pg_mv!(
        last,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 1, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 3, level: Some(3)]);

    /**
     * ignore_sight
     *
     * <00>01 02 03       00 01 02 03
     *  04 05 06 07       04 05 06 07
     *  08 09        =2  <08>09
     */
    assert_pg_mv!(
        last,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 2, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 0, level: Some(2)]);

    /**
     * ignore_sight
     *
     * <00>01 02 03       XX 00 01 02
     *  04 05 06 07       03 04 05 06
     *  08 09        =3  <07>08 09
     */
    assert_pg_mv!(
        last,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 3, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 1, level: Some(2)]);

    /**
     * ignore_sight
     *
     * <00>01 02 03       XX XX 00 01
     *  04 05 06 07      <02>03 04 05
     *  08 09        =8   06 07 08 09
     */
    assert_pg_mv!(
        last,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 8, ignore_sight: true, wrap: false],
        true,
        [fly_leaves: 2, level: Some(1)]);

    /**
     * ignore_sight
     *
     * <00>01 02 03       <00>01 02 03
     *  04 05 06 07        04 05 06 07
     *  08 09        =99   08 09
     */
    assert_pg_mv!(
        last,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [count: 99, ignore_sight: true, wrap: false],
        false,
        [fly_leaves: 0, level: Some(0)]);
}

#[test]
fn test_set_index() {
    /**
     * <00>     00
     *  01     <01>
     *  02      02
     *  03  =1  03
     */
    assert_pg!(
        set_index,
        [fly_leaves: 0, len: 4, level: None, sight_size: 1],
        [Index(1)],
        true,
        [fly_leaves: 0, level: Some(1)]);

    /**
     * <00>01 02 03       00 01 02 03
     *  04 05 06 07      <04>05 06 07
     *  08 09        =6   08 09
     */
    assert_pg!(
        set_index,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [Index(6)],
        true,
        [fly_leaves: 0, level: Some(1)]);

    /**
     * <00>01 02 03        00 01 02 03
     *  04 05 06 07        04 05 06 07
     *  08 09        =99  <08>09
     */
    assert_pg!(
        set_index,
        [fly_leaves: 0, len: 10, level: Some(0), sight_size: 4],
        [Index(99)],
        true,
        [fly_leaves: 0, level: Some(2)]);
}
