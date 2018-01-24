
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
    Cycle(bool), /* reverse */
}

#[derive(Clone, Debug, PartialEq)]
pub enum OptionName {
    PreDefined(PreDefinedOptionName),
    UserDefined(String),
}

iterable_enum!(PreDefinedOptionName =>
    AbbrevLength,
    AutoReload,
    AutoPaging,
    CenterAlignment,
    ColorError,
    ColorErrorBackground,
    ColorStatusBar,
    ColorStatusBarBackground,
    ColorWindowBackground,
    CurlConnectTimeout,
    CurlFollowLocation,
    CurlLowSpeedLimit,
    CurlLowSpeedTime,
    CurlTimeout,
    EmptyStatusFormat,
    FitTo,
    HistoryFile,
    HorizontalViews,
    LogFile,
    MaskOperator,
    PathList,
    PreFetchEnabled,
    PreFetchLimit,
    PreFetchPageSize,
    Reverse,
    Rotation,
    SkipResizeWindow,
    StatusBar,
    StdOut,
    StatusFormat,
    UpdateCacheAccessTime,
    TitleFormat,
    VerticalViews,
);


impl FromStr for PreDefinedOptionName {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        use self::PreDefinedOptionName::*;

        let result = match src {
            "abbrev-length" | "abbr-length"        => AbbrevLength,
            "auto-reload"                          => AutoReload,
            "auto-page" | "auto-paging" | "paging" => AutoPaging,
            "center" | "center-alignment"          => CenterAlignment,
            "curl-connect-timeout"                 => CurlConnectTimeout,
            "curl-follow-location"                 => CurlFollowLocation,
            "curl-low-speed-limit"                 => CurlLowSpeedLimit,
            "curl-low-speed-time"                  => CurlLowSpeedTime,
            "curl-timeout"                         => CurlTimeout,
            "empty-status-format"                  => EmptyStatusFormat,
            "fit" | "fit-to"                       => FitTo,
            "history-file"                         => HistoryFile,
            "horizontal-views"                     => HorizontalViews,
            "log-file" | "log"                     => LogFile,
            "mask-operator"                        => MaskOperator,
            "path"                                 => PathList,
            "pre-render"                           => PreFetchEnabled,
            "pre-render-limit"                     => PreFetchLimit,
            "pre-render-pages"                     => PreFetchPageSize,
            "reverse" | "rev"                      => Reverse,
            "rotation"                             => Rotation,
            "status-bar" | "status"                => StatusBar,
            "status-format"                        => StatusFormat,
            "stdout"                               => StdOut,
            "title-format"                         => TitleFormat,
            "vertical-views"                       => VerticalViews,
            "update-cache-atime"                   => UpdateCacheAccessTime,
            "window-background-color"              => ColorWindowBackground,
            "skip-resize-window"                   => SkipResizeWindow,
            "status-bar-color"                     => ColorStatusBar,
            "status-bar-background-color"          => ColorStatusBarBackground,
            "error-color"                          => ColorError,
            "error-background-color"               => ColorErrorBackground,
            _                                      => return Err(format!("Invalid option name: {}", src))
        };
        Ok(result)
    }
}

impl PreDefinedOptionName {
    pub fn is_for_curl(&self) -> bool {
        use self::PreDefinedOptionName::*;

        match *self {
            AbbrevLength | AutoReload | AutoPaging | CenterAlignment |
                ColorError | ColorErrorBackground | ColorStatusBar | ColorStatusBarBackground | ColorWindowBackground |
                FitTo | HorizontalViews | LogFile | MaskOperator |
                PathList | PreFetchEnabled | PreFetchLimit | PreFetchPageSize |
                Reverse | Rotation | SkipResizeWindow | StatusBar | StatusFormat | EmptyStatusFormat | StdOut | UpdateCacheAccessTime | TitleFormat | VerticalViews | HistoryFile => false,
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
