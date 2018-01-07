# chrysoberyl

The controllable image viewer


# Features

- Key mapping
- Mouse mapping (+ with screen region)
- Multi control source (stdin, fifo file, and user specified command's stdout)
- Multi cell view
- Many operation commands
- Support HTTP(s)
- Support Archive (zip, lha, rar, tar.gz)
- Color config (window, statusbar, error text)
- Shuffle
- Directory expanding
- Cherenkov
- Shell script friendly output (stdout)


## Multi cell view

Press `3v2V` for 3x2.

[Multi cell view](http://gyazo.snca.net/2017/04/11-191950-d08c6328e4315c20fb705933bcde6dd4.png)


## Cherenkov

Map to button with `@map mouse 2 @cherenkov --color red --radius 0.02`.
And click with wheel.

[Cherenkoved](http://gyazo.snca.net/2017/04/11-192852-ce7d9141eb69efce2c9e67e516dff69d.png)

Original -> Cherenkoved


# Operation commands

You can use below commands on STDIN of chrysoberyl.


## @cherenkov [(--radius|-r) <RADIUS>] [(--random-hue|--hue|-h) <HUE>] [(--spokes|-s) <SPOKES>] [-x <X>] [-y <Y>] [(-c|--color) <CSS_COLOR>]

Cherenkoves current image.

## @clear

Clear image entries.


## @clip

Clip by mouse selected rectangle.


## @count

Set `count` explicitly.


## @cycle <OPTION_NAME>

Cycle the value of `OPTION_NAME`.


## (@dec|@decrement| @decrease|@--) <OPTION_NAME>

Decrement the value of `OPTION_NAME`.


## @default

Load default configuration.


## @define-switch <SWITCH_NAME> <OPERATIONS_1> @@ <OPERATIONS_2> @@ ...

Define user switch.
Each operations are speparated by `@@`.

If you define as below...

```
@define-switch my-option ; @views 1 ; @disable reverse @@ ; @views 2 ; @enable reverse
```

then you can `@cycle my-option` to execute `; @views 1 ; @disable reverse` or `; @views 2 ; @enable reverse`.


## @delete <FILTER_EXPRESSION>

Delete the selected entries by `FILTER_EXPRESSION`.


## @disable <OPTION_NAME>

Disable `OPTION_NAME`.

`@disable reverse` equals `@set reverse false`.


## @draw

Redraw image.


## @editor [(--file|-f) <PATH>] [(--session|-s) <SESSION>] [<COMMAND_LINE>]

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


## @enable <OPTION_NAME>

Enable `OPTION_NAME`.

`@enable reverse` equals `@set reverse true`.


## @expand [--recursive|-r|--rec] [<PATH>]

Add directory entries (image) on `PATH`.


## @file (move|copy) [--fail|-f] [--overwrite|-o] [--new|--new-file-name|-n] [(--size|-s) <SIZE>] <DIRECTORY_PATH>

Move or copy the current image file to `DIRECTORY_PATH`.

### If destination file exists...

if `--fail` is given, then file operation fails.
if `--new` is given, then file operation succeeds with generated new file name.
if `--overwrite` is given, then overwrites destination file.


## @fill [(--shape|-s) <SHAPE>] [(--cell-index|-i) <CELL_INDEX>] [(--region|-r) <REGION>] [(--color|-c) <CSS_COLOR>] [--mask|-m]

Fill the shape.

You can use `@fill` with mapping.

```
@map region button-3 @fill --mask --filler circle --color red
```



## @filter [(--dynamic|-d)] [(--static|-s)] <FILTER_EXPRESSION>

Filter entries.



## (@first|@f) [--wrap|-w] [--archive|-a] [--ignore-views|-i] [<COUNT>]

Show `COUNT`th entry.


## @fly-leaves <NUMBER>

Insert `NUMBER` [flyleaves](https://en.wikipedia.org/wiki/Book_design#Front_cover,_spine,_and_back_cover_of_the_dust-jacket).


## @fragile <PATH>

Make fragile operation file.
You can write operation commands on this file to operate chrysoberyl.


## @go <PATH> [<INDEX>]

Show the entry.
`PATH` is entry path/URL.
`INDEX` is page index of archive/PDF.
This operation does not add any entry.


## (@inc|@increment|@increase|@++) <OPTION_NAME>

Increment the value of `OPTION_NAME`.


## @input

Feed input event.


## @kill-timer <NAME>

Kill a timer named `NAME`.


## (@last|@l) [--wrap|-w] [--archive|-a] [--ignore-views|-i] [<COUNT>]

Show `COUNT`th (from last) entry.


## @load [(--search-path|-p)] <PATH>

Load operation script file.
If `--search-path` is given, load from `search-path`.


## @map input [(--region|-r) <REGION>] <INPUT> <OPERATION>

Key/Mouse mapping.
See STDOUT of chrysoberyl for `INPUT`.


## @map event [--once|o] [(--repeat|-r) <TIMES>] <EVENT_NAME> <OPERATION>

When `EVENT_NAME` is fired, execute `OPERATION`


## @map region <MOUSE_BUTTON> <OPERATION>

## @meow

Meow


## @move-again [--wrap|-w] [--archive|-a] [--ignore-views|-i] [<COUNT>]

Move again by previous method.


## @multi [(--async|-a)] [(--sync|-s)] <SEPARATOR> <OPERATIONS_1> [<SEPARATOR> <OPERATIONS_2>]...

Execute multiple operations at once.

```
@multi <> @first 42 <> @views 2 <> @shell xmessage "at 42"
```


## ";" <OPERATIONS_1> [";" <OPERATIONS_2>]...

Same as `@multi`.

```
; @first 42 ; @views 2 ; @shell xmessage "at 42"
```


## (@next|@n) [--wrap|-w] [--archive|-a] [--ignore-views|-i] [<COUNT>]

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


## (@prev|@p| @previous) [--wrap|-w] [--archive|-a] [--ignore-views|-i] [<COUNT>]

Show `COUNT`th previous entry.


## @push [--meta <NAME>"="<VALUE>]... [--force|-f] <PATH_OR_URL>

Add a entry.


## @push-archive [--meta <NAME>"="<VALUE>]... [--force|-f] <PATH>

Add a archive file.


## (@push-directory|@push-dir) [--meta <NAME>"="<VALUE>]... [--force|-f] <DIRECTORY>

Add the images that the `DIRECTORY` contains.


## @push-image [--meta <NAME>"="<VALUE>]... [--force|-f] [--expand|-e] [--expand-recursive|-E] <PATH>

Add a image.
If `--expand` (or `--expand-recursive`) is given, add the images that the directory of `PATH` contains.


## @push-next

Add a next file.


## @push-pdf

Add a PDF file.


## (@push-previous|@push-prev)

Add a previous file.

## @push-url [--meta <NAME>"="<VALUE>]... [--force|-f] [(--type|-t|--as) <TYPE>] <URL>

Add a URL to image/PDF/archive.

### Type

- image
- archive
- pdf


## @quit

See you.


## (@random|@rand)

Show a entry randomly.


## (@refresh|@r)

Refresh something.


## @reset-image

Remove any effects (cherenkov etc).


## @save [(--target|-t) <TARGET>] <PATH>

Write the session data to `PATH`.
You can `@load` `PATH` file to restore current session.


## @scroll <DIRECTION> [(-s|--size) <SIZE>]

Scroll image.
If no `SIZE` is given, scroll size is `1.0`.

### DIRECTION

- left
- up
- right
- down


## @search [-b|--backward] [(-c|--color) <CSS_COLOR>] <TEXT>

Search text with `TEXT`, and highlights them.


## @set <OPTION_NAME> <VALUE>

Set option value.


## @set-env [--prefix|-p] [--system-prefix|-P] <NAME> <VALUE>

Set ENV.
`--prefix` adds `CHRY_X_` to `NAME`.
`--system-prefix` adds `CHRY_` to `NAME`.


## (@set-by-count|@set-count) <OPTION_NAME>

Set `COUNT` as value to `OPTION_NAME`.


## @shell [--async|-a] [--sync|-s] [(--session|-S) <SESSION>]... [--operation|-o] [--no-operation|-O] [--search-path|-p] <COMMAND> <COMMAND_ARG1>...

Execute shell command.
If `--operation` is given, chrysoberyl read the STDOUT of the command as operation commands.


## @shell-filter [--search-path|-p] <COMMAND> <COMMAND_ARG1>...

Execute `COMMAND` and pass the STDOUT of chrysoberyl to the `COMMAND` STDIN.


## @show [--wrap|-w] [--archive|-a] [--ignore-views|-i] [<COUNT>]

Show the entry.
This operation does not consider `views`.

## @shuffle

Shuffle entries.


## @sort

Sort entries.


## @timer [(--repeat|-r) <TIME>] [--infinity|-i] <NAME> <INTERVAL_SEC> <OPERATION>...

Execute `OPERATION` repeatedly.

```
@timer -i next-page 1.5 @next
```

You can use `@kill-timer` to stop this task.


## @toggle <OPTION_NAME>

Toggle between option values (true/false).


## @unclip

Restore from `@clip`.


## @undo [<COUNT>]

Undo some operations (fill/cherenkov).


## @unless <FILTER_EXPRESSION> <OPERATION>...

If the result of `FILTER_EXPRESSION` evaluation is false, execute the operation.


## @unmap (input|region|event) <NAME>

Unmap the `@map`ped.


## @unset <OPTION_NAME>

Set default value.


## @update [--image] [--image-options|-o] [--label|-l] [--message|-m] [--pointer|-p]

For developper.


## @user

DEPRECATED.


## @views <COLUMNS> [<ROWS>]

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
BoolVariable ← 'animation'
```
