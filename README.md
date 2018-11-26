# chrysoberyl

The controllable image viewer


# Features

- Key mapping
- Mouse mapping (+ with screen region)
- Multi control source (stdin, fifo file, and user specified command's stdout)
- Multi cell view
- Many operation commands
- Support HTTP(s)
- Support Archive (zip, lha, rar, tar.gz and more)
- Color config (window, statusbar, error text)
- Shuffle
- Directory expanding
- Cherenkov
- Shell script friendly output (stdout)


## Multi cell view

Press `3v2V` for 3x2.

![Multi cell view](http://gyazo.snca.net/2017/04/11-191950-d08c6328e4315c20fb705933bcde6dd4.png)


## Cherenkov

Map to button with `@map input button-2 @cherenkov --color blue --radius 0.02`.
And click with wheel.

![Cherenkoved](http://gyazo.snca.net/2017/04/11-192852-ce7d9141eb69efce2c9e67e516dff69d.png)

Original -> Cherenkoved


# Command line

```
chrysoberyl <OPTIONS>... (<FILE>|<DIRECTORY>)
chrysoberyl <OPTIONS>... <OPERATION_COMMAND>... ["@@" (<FILE>|<DIRECTORY>)...]
chrysoberyl (--print-path|-v)
chrysoberyl (--version|-v)
chrysoberyl (--help|-h)
```

## Options

| Name                                    | Description                                         |
| ----                                    | ------------                                        |
| `--silent`                              | Suppress stdout                                     |
| `--shuffle\|-z`                         | Shuffle entries                                     |
| `--expand\|e`                           | Expand directory (`@expand`)                        |
| `--expand-recursive\|-E`                | Expand directory (`@expand`)                        |
| `(--max-curl-threads\|-t) <N_THREADS>`  | Set maximum curl threads                            |
| `(--input\|-i) <PATH>`                  | Create the file to input operation commands         |
| `--encoding <ENCODING>`                 | Set filename encoding for archives (zip, rar, ...). |
| `--role <WINDOW_ROLE>`                  | Set window role                                     |
| `--use-gtk-theme`                       | Use GTK theme                                       |


## Example

Open a PDF file.

```
$ chrysoberyl ~/my-books/rust-nomicon.pdf
```

Set some options (operation commands), then open a PDF file.

```
$ chrysoberyl @@views 2 @@set reverse @@ ~/my-books/rust-nomicon.pdf
```

Print the file or directory path used by chrysoberyl.

```
$ chrysoberyl --print-path
```



# Operation commands

You can use below commands on STDIN of chrysoberyl.


## @backword

Backword history

## @cd <DIRECTORY>

Change working directory.


## @cherenkov [(--radius|-r) <RADIUS>] [(--random-hue|--hue|-h) <HUE>] [(--spokes|-s) <SPOKES>] [-x <X>] [-y <Y>] [(-c|--color) <CSS_COLOR>] [--seed|-S]

Cherenkoves current image.


## @cherenkov-reset

Reset.

## @clear

Clear image entries.


## @clip [<X> [<Y> [<WIDTH> [<HEIGHT>]]]]

Clip by mouse selected rectangle.


## @command-line <INITIAL_COMMAND>

Show command line with `<INITIAL_COMMAND>`.


## @controller-file <FILE>

Open `<PATH>` file to control chrysoberyl.

## @controller-fifo <FILE>

Open `<PATH>` fifo file to control chrysoberyl.

## @controller-socket [--as-binary|-b] <FILE>

Open `<PATH>` socket to control chrysoberyl.


## @copy-to-clipboard [--meta <KEY_VALUE>] [--primary|-1|--secondary|-2|--clipboard]

Copy currently viewing image to clipboard.


## @count

Set `count` explicitly.


## @cycle <OPTION> [<CANDIDATES>...]

Cycle the value of `OPTION`.


## (@dec|@decrement|@decrease|@--) <OPTION>

Decrement the value of `OPTION`.


## @default

Load default configuration.


## @define-switch <SWITCH_NAME> <OPERATIONS_1> @@ <OPERATIONS_2> @@ ...

Define user switch.
Each operations are speparated by `@@`.

If you define as below...

```
@define-switch my-option ; @views 1 ; @disable reverse @@ ; @views 2 ; @enable reverse
```

then you can use `@cycle my-option` to execute `; @views 1 ; @disable reverse` or `; @views 2 ; @enable reverse`.


## @delete <FILTER_EXPRESSION>

Delete the selected entries by `FILTER_EXPRESSION`.


## @disable <OPTION>

Disable `OPTION`.

`@disable reverse` equals `@set reverse false`.


## @draw

Redraw image.


## @editor [(--file|-f) <PATH>] [(--session|-s) <SESSION>] [--comment-out|-c] [--freeze|-f] [<COMMAND_LINE>]

### SESSION

- options
- entries
- paths
- position
- mappings
- envs
- filter
- reading
- all


## @enable <OPTION>

Enable `OPTION`.

`@enable reverse` equals `@set reverse true`.


## @eval <OPERATION>...

Expand ENV variables in `<OPERATION>...`.

e.g)

```
@query @eval @mark $CHRY_QUERY
@query @eval @jump --load $CHRY_QUERY
```


## @expand [--recursive|-r|--rec] [<PATH>]

Add directory entries (image) on `PATH`.


## @file-move [--fail|-f] [--overwrite|-o] [--new|--new-file-name|-n] [(--size|-s) <SIZE>] [--as-filepath|-F] <DIRECTORY> [<FILE>]

Move the current image file to `DIRECTORY`.

### If destination file exists...

if `--fail` is given, then file operation fails.
if `--new` is given, then file operation succeeds with generated new file name.
if `--overwrite` is given, then overwrites destination file.


## @file-copy [--fail|-f] [--overwrite|-o] [--new|--new-file-name|-n] [(--size|-s) <SIZE>] [--as-filepath|-F] <DIRECTORY> [<FILE>]

Copy the current image file to `DIRECTORY`.
See `@file-move` to get more information.


## @fill [(--shape|-s) <SHAPE>] [(--cell-index|-i) <CELL_INDEX>] [(--region|-r) <REGION>] [(--color|-c) <CSS_COLOR>] [(--operator|-o) <OPERATOR>] [--mask|-m]

Fill the shape.

You can use `@fill` with mapping.

```
@map region button-3 @fill --mask --filler circle --color red
```



## @filter [(--dynamic|-d)] [(--static|-s)] <FILTER_EXPRESSION>

Filter entries.



## @fire <MAPPED_TYPE> <NAME>

Fire mapped input/event.
`@fire` does not support `region` mapping.


## (@first|@f) [--wrap|-w] [--archive|-a] [--ignore-views|-i] [<COUNT>]

Show `COUNT`th entry.


## @flush-buffer

Pull all buffered entries.
See `$CHRY_REMOTE_BUFFER`, if you want to get the number of buffered entries.


## @fly-leaves <NUMBER>

Insert `NUMBER` [flyleaves](https://en.wikipedia.org/wiki/Book_design#Front_cover,_spine,_and_back_cover_of_the_dust-jacket).


## @forwrd

Forword history


## @fragile <PATH>

Make fragile operation file.
You can write operation commands on this file to operate chrysoberyl.


## @go <PATH> [<INDEX>]

Show the entry.
`PATH` is entry path/URL.
`INDEX` is page index of archive/PDF.
This operation does not add any entry.


## (@inc|@increment|@increase|@++) <OPTION>

Increment the value of `OPTION`.


## @input <INPUT1> [<INPUT2>...]

Feed input event.


## @kill-timer <NAME>

Kill a timer named `NAME`.


## (@last|@l) [--wrap|-w] [--archive|-a] [--ignore-views|-i] [<COUNT>]

Show `COUNT`th (from last) entry.


## @load [(--search-path|-p)] <PATH>

Load operation script file.
If `--search-path` is given, load from `search-path`.


## @load-ui [(--search-path|-p)] <PATH>

Load `*.glade` file.
See [Glade - A User Interface Designer](https://glade.gnome.org/).


## @map (input|event|region|operation) ...

### @map input [(--region|-r) <REGION>] <INPUT> <OPERATION>

Key/Mouse mapping.
See STDOUT of chrysoberyl for `INPUT`.


### @map event [--once|o] [(--repeat|-r) <TIMES>] <EVENT_NAME> <OPERATION>

When `EVENT_NAME` is fired, execute `OPERATION`


### @map region <MOUSE_BUTTON> <OPERATION>

For `@clip` and `@fill`.


## @message [--keep|-k] [<MESSAGE>]

If `<MESSAGE>` is not given, remove message.
If `--keep` is given, keep current message.


## @meow

Meow


## @move-again [--wrap|-w] [--archive|-a] [--ignore-views|-i] [<COUNT>]

Move again by previous method.


## @multi [--async|-a] [--sync|-s] <SEPARATOR> <OPERATIONS_1> [<SEPARATOR> <OPERATIONS_2>]...

Execute multiple operations at once.

```
@multi <> @first 42 <> @views 2 <> @shell xmessage "at 42"
```


## ";" <OPERATIONS_1> [";" <OPERATIONS_2>]...

Same as `@multi`.

```
; @first 42 ; @views 2 ; @shell xmessage "at 42"
```


## (@next|@n) [--wrap|-w] [--archive|-a] [--ignore-views|-i] [--forget|-f] [<COUNT>]

Show `COUNT`th next entry.


## @page [<PAGE_NUMBER>]

Show `COUNT` entry of the currently viewing archive/PDF.


## @pdf-index [--async|-a] [--sync|-s] [--operation|-o] [--no-operation|-O] [--separator <SEPARATOR>] [(--format|-f) <FORMAT>] <COMMAND> <COMMAND_ARG1>...

Execute shell `COMMAND`.
Chrysoberyl passes PDF Index data to the STDIN of the `COMMAND` process.

### FORMAT

- "1" | "one-line" | "one" | "o"
- "2" | "two-lines" | "two" | "t"
- "indented" | "indent" | "i"


## @link-action [<OPERATION>...]

e.g)

```
@map input button-1 @link-action @next
```

## (@prev|@p|@previous) [--wrap|-w] [--archive|-a] [--ignore-views|-i] [--forget|-f] [<COUNT>]

Show `COUNT`th previous entry.


## @push [--meta <KEY_VALUE>]... [--force|-f] [--show|-s] <PATH>

Add a entry.
`PATH` is URL, file or directory.


## @push-archive [--meta <KEY_VALUE>]... [--force|-f] [--show|-s] <FILE>

Add a archive file.


## @push-clipboard [--meta <KEY_VALUE>]... [--operation|-o] [--show|-s] [--primary|-1|--secondary|-2|--clipboard]

Add a clipboard image.


## (@push-directory|@push-dir) [--meta <KEY_VALUE>]... [--force|-f] [--show|-s] <DIRECTORY>

Add the images that the `DIRECTORY` contains.


## @push-image [--meta <KEY_VALUE>]... [--force|-f] [--show|-s] [--expand|-e] [--expand-recursive|-E] <FILE>

Add a image.
If `--expand` (or `--expand-recursive`) is given, add the images that the directory of `PATH` contains.


## @push-next [--show|-s]

Add a next file.


## @push-pdf [--show|-s]

Add a PDF file.


## (@push-previous|@push-prev) [--show|-s]

Add a previous file.

## @push-url [--meta <KEY_VALUE>]... [--force|-f] [--show|-s] [(--type|-t|--as) <TYPE>] <URL>

Add a URL to image/PDF/archive.

### Type

- image
- archive
- pdf


## @queue <OPERATION>...

Queue the operation to processsing queue.

## @quit

See you.


## (@random|@rand)

Show a entry randomly.


## @record <OPERATION>...

Record to history


## (@refresh|@r) [--image|-i]

Refresh something.
If `--image` is given, clear the caches for currently viewing entries.


## @reset-image

Remove any effects (cherenkov etc).


## @save [(--target|-t) <TARGET>] [--freeze|-f] <PATH>

Write the session data to `PATH`.
You can `@load` `PATH` file to restore current session.


## @scroll [(--size|-s) <SIZE>] [--crush|-c] [--reset|-r] (up|down|left|right) [<OPERATION>...]

Scroll image.
If no `SIZE` is given, scroll size is `1.0`.


## @search [--backward|-b] [(-c|--color) <CSS_COLOR>] <TEXT>

Search text with `TEXT`, and highlights them.


## @set <OPTION> <VALUE>

Set option value.


## @set-env [--prefix|-p] [--system-prefix|-P] <NAME> <VALUE>

Set ENV.
`--prefix` adds `CHRY_X_` to `NAME`.
`--system-prefix` adds `CHRY_` to `NAME`.


## (@set-by-count|@set-count) <OPTION>

Set `COUNT` as value to `OPTION`.


## @shell [--async|-a] [--sync|-s] [(--session|-S) <SESSION>]... [--freeze|-f] [--operation|-o] [--no-operation|-O] [--search-path|-p] [--as-binary|-b] <COMMAND> <COMMAND_ARG1>...

Execute shell command.
If `--operation` is given, chrysoberyl read the STDOUT of the command as operation commands.


## @shell-filter [--search-path|-p] <COMMAND> <COMMAND_ARG1>...

Execute `COMMAND` and pass the STDOUT of chrysoberyl to the `COMMAND` STDIN.


## @show [--wrap|-w] [--archive|-a] [--ignore-views|-i] [<COUNT>]

Show the entry.

## @shuffle

Shuffle entries.


## @sort [--accessed|-a] [--created|-c] [--modified|-m] [--reverse|-r] [--fix|-f] [<COMMAND> <COMMAND_ARG1>...]

Sort entries.


## @timer [(--name|-n) <NAME>] [(--repeat|-r) <TIME>] [--infinity|-i] [--async|-a] [--sync|-s] <INTERVAL_SEC> <OPERATION>...

Execute `OPERATION` repeatedly.

```
@timer -i next-page 1.5 @next
```

You can use `@kill-timer` to stop this task.


## @toggle <OPTION>

Toggle between option values (true/false).


## @unclip

Restore from `@clip`.


## @undo [<COUNT>]

Undo some operations (fill/cherenkov).


## @unless <FILTER_EXPRESSION> <OPERATION>...

If the result of `FILTER_EXPRESSION` evaluation is false, execute the operation.


## @unmap (input|region|event|operation) <NAME>

Unmap the `@map`ped.


## @unset <OPTION>

Set default value.


## @update [--image] [--image-options|-o] [--status|-s] [--message|-m] [--pointer|-p]

For developper.


## @user

DEPRECATED.


## (@views|@v) [--ignore-views] <COLUMNS> [<ROWS>]

Set the number of view cells.
The default of `ROWS` is 1.

If you want to read a manga, maybe you should...

```
@views 2
```

## @when <FILTER_EXPRESSION> <OPERATION>...

If the result of `FILTER_EXPRESSION` evaluation is true, execute the operation.


## @write [--index <CELL_INDEX>] <PATH>

Write current entry image to `PATH`.
`@write` generate the image that is all effects applyed.


## <KEY_VALUE> format

> <KEY>=<VALUE>

e.g.

> key=value


# Options

| Name                 | Type                                                          | Default                              | Description                                                                                                      |
| ----                 | ----                                                          | ----                                 | ----                                                                                                             |
| abbrev-length        | unsigned integer                                              |                                      | Max length for `CRHY_ABBREV_PATH`                                                                                |
| animation            | boolean                                                       |                                      | Support animation GIF                                                                                            |
| auto-reload          | boolean                                                       |                                      | Reload current viewingly images when they are updated.                                                           |
| auto-paging          | no/always/smart                                               |                                      |                                                                                                                  |
| curl-connect-timeout | unsigned integer                                              | 10                                   | cURL option (seconds)                                                                                            |
| curl-follow-location | boolean                                                       | true                                 | cURL option                                                                                                      |
| curl-low-speed-limit | unsigned integer                                              |                                      | cURL option                                                                                                      |
| curl-low-speed-time  | unsigned integer                                              |                                      | cURL option                                                                                                      |
| curl-timeout         | type: unsigned integer                                        | none                                 | cURL option (seconds)                                                                                            |
| empty-status-format  | string-or-file                                                |                                      | Status bar format for empty. Give a string or a mruby script file path (e.g. `@~/.config/chrysoberyl/status.rb`) |
| fit-to               | width/height/original/original-or-cell/cell/XXX%/WIDTHxHEIGHT | cell                                 |                                                                                                                  |
| idle-time            | real number                                                   | 0.25                                 | Delay time for `idle` event                                                                                      |
| history-file         | path                                                          | none                                 | STDIN input history file                                                                                         |
| horizontal-flip      | boolean                                                       |                                      | Flip images horizontally                                                                                         |
| horizontal-views     | unsigned integer                                              |                                      | Number of horizontal cells (1 or more)                                                                           |
| initial-positoin     | top-left/top-right/bottom-left/bottom-right                   |                                      | Initial image position in cell                                                                                   |
| log-file             | path                                                          | none                                 | Path for log                                                                                                     |
| mask-operator        | mark operators                                                |                                      |                                                                                                                  |
| path                 | path                                                          |                                      | Script search path                                                                                               |
| pointer-hide-delay   | unsinged integer                                              | none                                 | Time to hide pointer (mouse cursor)
| pre-render           | boolean                                                       | true                                 |                                                                                                                  |
| pre-render-limit     | unsigned integer                                              | 100                                  | 1 or more                                                                                                        |
| pre-render-pages     | unsigned integer                                              | 5                                    | 1 ore more                                                                                                       |
| pre-render-stages    | unsigned integer                                              | 3                                    | 1 ore more                                                                                                       |
| reverse              | boolean                                                       | false                                |                                                                                                                  |
| rotation             | 0/1/2/3                                                       |                                      |
| screen               | main/command-line/log-view/ui                                 |                                      |                                                                                                                  |
| status-bar           | boolean                                                       | true                                 |                                                                                                                  |
| status-bar-align     | left/center/right                                             | center                               |                                                                                                                  |
| status-bar-height    | unsigned integer                                              | none                                 | 1 or more                                                                                                        |
| status-format        | string-or-file                                                |                                      | Give a string or a mruby script file path (e.g. `@~/.config/chrysoberyl/status.rb`)                              |
| stdout               | boolean                                                       |                                      |                                                                                                                  |
| style                | string-or-file                                                |                                      | Give a CSS string or a CSS file path (e.g. `@~/.config/chrysoberyl/style.css`)                                   |
| title-format         | string-or-file                                                |                                      | Give a string or a mruby script file path (e.g. `@~/.config/chrysoberyl/title.rb`)                               |
| vertical-flip        | boolean                                                       |                                      | Flip images vertically                                                                                           |
| vertical-views       | unsigned integer                                              | Number of vertical cells (1 or more) |
| update-cache-atime   | boolean                                                       | false                                |                                                                                                                  |
| watch-files          |                                                               |                                      | Fire `file-changed` event when currently viewing images are updated                                              |
| skip-resize-window   | unsigned integer                                              |                                      | TODO                                                                                                             |

## Mask operators

- clear
- source
- over
- in
- out
- atop
- dest
- dest-over
- dest-in
- dest-out
- dest-atop
- xor
- add
- saturate
- multiply
- screen
- overlay
- darken
- lighten
- color-dodge
- color-burn
- hard-light
- soft-light
- difference
- exclusion
- hsl-hue
- hsl-saturation
- hsl-color
- hsl-luminosity

## Note

You can use `@editor` to see current option values.

```
@editor --session all $EDITOR
```

## Supportted operations

unsigned integer
:   @set, @unset, @increase, @decrease, @cycle


# Events

- at-first
- at-last
- download-all
- error
- file-changed
- idle
- initialize
- invalid-all
- mapped-input
- quit
- resize-window
- show-image-pre
- show-image
- spawn
- void

# Filter Expression

```
Expr ← Block | Bool | Cond | Logic | 'not' Expr
Block ← '(' Expr ')' | '{' Expr '}'
Logic ← Bool LogicOp Expr
Bool ← Compare | BoolVariable | 'true' | 'false'
Cond ← 'if' Expr Expr Expr | 'when' Expr Expr | 'unless' Expr Expr
BoolOp ← 'and' | 'or'
Compare ← Value CmpOp Value
CmpOp ← '<' | '<=' | '>' | '>=' | '=' | '==' | '!=' | '=*' | '!*'
Value ← Glob | Integer | Variable
Variable ← 'type' | 'width' | 'height' | 'path' | 'ext' | 'extension' | 'dimensions' | 'name' | 'filesize' | 'page' | 'pages' | 'real-pages' | 'ratio'
Glob ← '<' string '>'
BoolVariable ← 'animation' | 'active' | 'valid'
```

# User Defined UI

Chrysoberyl supports `*.glade` file.
You can execute `@load-ui` to load the UI you defined and execute `@set screen ui` to show the UI.

When you click  a named widget or type `Return` on Entry), chrysoberyl fire the event.
(If you name a widget with `foo`, `ui-foo` will be fired)
Some events have the environment variable `$CHRY_VALUE`.

## Supported widget types

- Button
- ComboBoxText
- Entry
- RadioButton
- Scale
- Switch
