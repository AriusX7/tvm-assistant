<h1 align="center"><a href=".">Main Index</a></h1>

<p align="center">
  <a href="#introduction">Introduction</a>
  •
  <a href="#command-usage">Command Usage</a>
  •
  <a href="#syntax">Syntax</a>
</p>

## Introduction

In order to help display mass-quotes and other large text posts nicely, TvM Assistant has a command called `format` which adds the following formatting features to text posts in addition to the normal features support by Discord:

- Multi-level nested quotes
- Six heading levels
- Ordered and unordered lists
- Tables
- Hyperlinks
- Superscript text
- Horizontal rules

`format` command doesn't support the following Discord Markdown features:

- Underlines
- Spoilers
- Syntax highlighting in code blocks

The syntax to add all of this can easily lead to formatting issues. You can use one of these websites to see a live preview of your text:

- [StackEdit](https://stackedit.io/) - Doesn't support multi-word superscript
- [Dillinger](https://dillinger.io/) - Doesn't support superscript and nested quotes
- [Jbt](https://jbt.github.io/markdown-editor/) - Doesn't support superscript

## Command Usage

The `format` command can be used by simply adding `!format` in front of the text. The bot then parses the text and converts it into a formatted JPEG and a similarly formatted PDF. Both the files are then sent in the text channel the command is used in. The JPEG allows other players to read the post quickly while the PDF is handy for copy-pasting text and clicking on links, hyperlinks, etc.

**Note 1:** When parsing the text, all extra and unneeded whitespace is removed.

**Note 2:** The command is relatively slow. It can take anywhere between 5 seconds to 10 seconds to send the output files.

You can read more about the formatting syntax in [the next section](#syntax).

### Example Command

```placeholder
!format
# Level 1 Heading
## Level 2 Heading
###### Level 6 Heading

> This is a single quote.
>> This quote is inside the first quote.
>>> This is inside the second quote. Three nest layers!

> Back to single quote.

And finally, no quotes.

Now, we'll add a [hyperlink](https://www.google.com/).

Let's create at a table:

some|header|labels
:---|:--:|---:
Left-justified|center-justified|right-justified
a|b|c
d|e|f
```

### Output

<img src="https://i.imgur.com/aq5F8MR.png" alt="Output Image">

## Syntax

The syntax to add all the supported features, including Discord Markdown features, is documented below.

### Bold, Italics, Strikethrough and Superscript

To make text bold, put `**` or `__` around it. For example, \*\*bold\*\* becomes **bold**.

To italicize text, put `*` or `_` around it. For example, \_italics\_ becomes _italics_.

To strikethrough the text, put `~` or `~~` around it. For example, both \~strikethrough\~ and \~\~strikethrough\~\~ become ~~strikethrough~~.

To superscript text, add `^` around it.

```md
x^2^ + y^2^ = z^2^

This sentence includes super^script.^

This sentence^has superscript with^ multiple words.
```

produces

<img src="https://i.imgur.com/mDP6sOY.png" alt="Superscript">

**Underlined text** is not supported by the `format` command. `__text__` results in __text__.

### Inline Code and Code Blocks

Text can be converted into inline code by adding backticks (\`) around it. For example, \`inline code\` becomes `inline code`.

Blocks of code are fenced by lines with three (or more) backticks ```.

The following

````md
```
fn main() {
    println!("Hello, world!");
}
```
````

produces

```placeholder
fn main() {
    println!("Hello, world!");
}
```

**Note:** `format` command doesn't support syntax highlighting. You can specify identifier of a language after the opening backticks, but it'll not have any effect.

### Quotes

To make text appear as a quote, put a `>` before it. To nest a quote inside it, put `>>` before the text that needs to be nested. Increase number of `>` to increase the nest level.

The following

```placeholder
> This is a single quote.
>> This is nested inside the single quote.
>>> This is nested inside the nested quote.

> We're back to a single quote.

And finally, no quote.
```

produces

<img src="https://i.imgur.com/rx10Efk.png" alt="Quotes">

To decrease the nest level, you need to leave a blank line, like in the example above.

### Headings

`format` command supports six levels of headings. You can specify text as level one heading by add a pound symbol (`#`) in front of it. Make sure to leave a space between `#` and the text.

To decrease heading level, increase number of pound symbols.

The following

```md
# Heading level one
## Heading level two
### Heading level three
#### Heading level four
##### Heading level five
###### Heading level six
```

produces

<img src="https://i.imgur.com/xNg6rbw.png" alt="Headings">

### Lists

Both ordered and unordered lists are supported by the `format` command. To create an unordered list, add one of `-`, `*` and `+` in front of the item text.

The following

```md
- Town
- Mafia
- Third Party
```

produces

- Town
- Mafia
- Third Party

To create an ordered list, add item number in front of item text.

The following

```md
1. Town
2. Mafia
3. Third Party
```

produces

1. Town
2. Mafia
3. Third Party

### Tables

Tables have a complicated structure. The following is taken from [here](https://github.com/adam-p/markdown-here/wiki/Markdown-Cheatsheet#tables).

```md
Colons can be used to align columns.

| Tables        | Are           | Cool  |
| ------------- |:-------------:| -----:|
| col 3 is      | right-aligned | $1600 |
| col 2 is      | centered      |   $12 |
| zebra stripes | are neat      |    $1 |

There must be at least 3 dashes separating each header cell.
The outer pipes (|) are optional, and you don't need to make the
raw Markdown line up prettily. You can also use inline Markdown.

Markdown | Less | Pretty
--- | --- | ---
*Still* | `renders` | **nicely**
1 | 2 | 3
```

The above text produces

Colons can be used to align columns.

| Tables        | Are           | Cool  |
| ------------- |:-------------:| -----:|
| col 3 is      | right-aligned | $1600 |
| col 2 is      | centered      |   $12 |
| zebra stripes | are neat      |    $1 |

There must be at least 3 dashes separating each header cell.
The outer pipes (|) are optional, and you don't need to make the
raw Markdown line up prettily. You can also use inline Markdown.

Markdown | Less | Pretty
--- | --- | ---
*Still* | `renders` | **nicely**
1 | 2 | 3

### Hyperlinks and Horizontal Rules

To add a hyperlink, wrap the text to be hyperlinked within `[]`, followed up by `()` containing the link.

The following

```md
[Click here](https://www.google.com/) to go to Google.
```

produces

[Click here](https://www.google.com/) to go to Google.

Horizontal rules can be added by typing three or more asterisks (`*`), dashes (`-`) or underscores (`_`) in a new line.

The following

```md
There will be a horizontal rule below this line.

---

That was a horitzontal rule.
```

produces

There will be a horizontal rule below this line.

---

That was a horitzontal rule.
