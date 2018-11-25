
use std::str::FromStr;


#[derive(Clone, Debug, PartialEq)]
pub enum OptionUpdater {
    Set(String),
    Unset,
    Enable,
    Disable,
    Toggle,
    SetByCount,
    Increment(usize),
    Decrement(usize),
    Cycle(bool, Vec<String>), /* reverse, candidates */
}

#[derive(Clone, Debug, PartialEq)]
pub enum OptionName {
    PreDefined(PreDefinedOptionName),
    UserDefined(String),
}

iterable_enum!(PreDefinedOptionName =>
    AbbrevLength,
    Animation,
    AutoPaging,
    AutoReload,
    ColorLink,
    CurlConnectTimeout,
    CurlFollowLocation,
    CurlLowSpeedLimit,
    CurlLowSpeedTime,
    CurlTimeout,
    EmptyStatusFormat,
    FitTo,
    Freeze,
    HistoryFile,
    HorizontalFlip,
    HorizontalViews,
    IdleTime,
    InitialPosition,
    LogFile,
    MaskOperator,
    PathList,
    PreFetchEnabled,
    PreFetchLimit,
    PreFetchPageSize,
    PreFetchStages,
    Reverse,
    Rotation,
    Screen,
    SkipResizeWindow,
    StablePush,
    StatusBar,
    StatusBarAlign,
    StatusBarHeight,
    StatusFormat,
    StdOut,
    Style,
    TimeToHidePointer,
    TitleFormat,
    UpdateCacheAccessTime,
    VerticalFlip,
    VerticalViews,
    WatchFiles,
);


impl FromStr for PreDefinedOptionName {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        use self::PreDefinedOptionName::*;

        let result = match src {
            "abbrev-length" | "abbr-length"        => AbbrevLength,
            "animation" | "anim"                   => Animation,
            "auto-reload"                          => AutoReload,
            "auto-page" | "auto-paging" | "paging" => AutoPaging,
            "curl-connect-timeout"                 => CurlConnectTimeout,
            "curl-follow-location"                 => CurlFollowLocation,
            "curl-low-speed-limit"                 => CurlLowSpeedLimit,
            "curl-low-speed-time"                  => CurlLowSpeedTime,
            "curl-timeout"                         => CurlTimeout,
            "empty-status-format"                  => EmptyStatusFormat,
            "fit-to" | "fit"                       => FitTo,
            "freeze" | "freezed"                   => Freeze,
            "history-file"                         => HistoryFile,
            "horizontal-flip"                      => HorizontalFlip,
            "horizontal-views"                     => HorizontalViews,
            "idle-time"                            => IdleTime,
            "initial-position"                     => InitialPosition,
            "log-file" | "log"                     => LogFile,
            "mask-operator"                        => MaskOperator,
            "path"                                 => PathList,
            "pointer-hide-delay"                   => TimeToHidePointer,
            "pre-render"                           => PreFetchEnabled,
            "pre-render-limit"                     => PreFetchLimit,
            "pre-render-pages"                     => PreFetchPageSize,
            "pre-render-stages"                    => PreFetchStages,
            "reverse" | "rev"                      => Reverse,
            "rotation"                             => Rotation,
            "screen"                               => Screen,
            "stable-push"                          => StablePush,
            "status-bar" | "status"                => StatusBar,
            "status-bar-align" | "status-align"    => StatusBarAlign,
            "status-bar-height" | "status-height"  => StatusBarHeight,
            "status-format"                        => StatusFormat,
            "style"                                => Style,
            "stdout"                               => StdOut,
            "title-format"                         => TitleFormat,
            "update-cache-atime"                   => UpdateCacheAccessTime,
            "vertical-flip"                        => VerticalFlip,
            "vertical-views"                       => VerticalViews,
            "watch-files"                          => WatchFiles,
            "skip-resize-window"                   => SkipResizeWindow,
            "link-color"                           => ColorLink,
            _                                      => return Err(format!("Invalid option name: {}", src))
        };
        Ok(result)
    }
}

impl PreDefinedOptionName {
    pub fn is_for_curl(&self) -> bool {
        use self::PreDefinedOptionName::*;

        match *self {
            AbbrevLength | Animation | AutoReload | AutoPaging |
                ColorLink |
                FitTo | Freeze | HorizontalFlip | HorizontalViews | IdleTime | InitialPosition | LogFile | MaskOperator |
                PathList | PreFetchEnabled | PreFetchLimit | PreFetchPageSize | PreFetchStages |
                Reverse | Rotation | Screen | SkipResizeWindow | StablePush | StatusBar | StatusBarAlign | StatusBarHeight | StatusFormat | EmptyStatusFormat | Style |
                StdOut | UpdateCacheAccessTime | TimeToHidePointer | TitleFormat | VerticalFlip | VerticalViews | WatchFiles | HistoryFile => false,
            CurlConnectTimeout | CurlFollowLocation | CurlLowSpeedLimit | CurlLowSpeedTime | CurlTimeout => true,
        }
    }
}


impl FromStr for OptionName {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        use self::OptionName::*;

        Ok({
            src.parse().map(PreDefined).unwrap_or_else(|_| {
                UserDefined(o!(src))
            })
        })
    }
}

impl Default for OptionName {
    fn default() -> Self {
        OptionName::PreDefined(PreDefinedOptionName::StatusBar)
    }
}
