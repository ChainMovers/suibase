---
hide:
  - toc
---

---

# Just Random Documentation For Test

This is a shameless copy from "mkdocs material" to show how sui-base will look like.

See here for original:
https://squidfunk.github.io/mkdocs-material/reference/

(This page will be deleted soon!)

## Configuration

This configuration enables syntax highlighting on code blocks and inline code 
blocks, and allows to include source code directly from other files. Add the 
following lines to `mkdocs.yml`:

``` yaml
markdown_extensions:
  - pymdownx.highlight:
      anchor_linenums: true
      line_spans: __span
      pygments_lang_class: true
  - pymdownx.inlinehilite
  - pymdownx.snippets
  - pymdownx.superfences
```

The following sections discuss how to use different syntax highlighting features
with [Pygments], the recommended highlighter, so they don't apply when using a
JavaScript syntax highlighter.

### Code copy button

[:octicons-tag-24: 9.0.0][Code copy button support] ·
:octicons-unlock-24: Feature flag

Code blocks can automatically render a button on the right side to allow the
user to copy a code block's contents to the clipboard. Add the following to
`mkdocs.yml` to enable them globally:

``` yaml
theme:
  features:
    - content.code.copy
```

  [Code copy button support]: https://github.com/squidfunk/mkdocs-material/releases/tag/9.0.0

??? info "Enabling or disabling code copy buttons for a specific code block"

    If you don't want to enable code copy buttons globally, you can enable them
    for a specific code block by using a slightly different syntax based on the
    [Attribute Lists] extension:

    ```` yaml
    ``` { .yaml .copy }
    # Code block content
    ```
    ````

    Note that the language shortcode which has to come first must now also be 
    prefixed by a `.`. Similarly, the copy button can also be disabled for a
    specific code block:

    ```` { .yaml .no-copy }
    ``` { .yaml .no-copy }
    # Code block content
    ```
    ````

### Code selection button :material-alert-decagram:{ .mdx-pulse title="Added on February 19, 2023" }

[:octicons-heart-fill-24:{ .mdx-heart } Sponsors only][Insiders]{ .mdx-insiders } ·
[:octicons-tag-24: insiders-4.32.0][Insiders] ·
:octicons-beaker-24: Experimental

Code blocks can include a button to allow for the selection of line ranges by
the user, which is perfect for linking to a specific subsection of a code block. This allows the user to apply [line highlighting] dynamically. Add the following
to `mkdocs.yml` to enable it globally:

``` yaml
theme:
  features:
    - content.code.select
```

??? info "Enabling or disabling code selection buttons for a specific code block"

    If you don't want to enable code selection buttons globally, you can enable 
    them for a specific code block by using a slightly different syntax based on 
    the [Attribute Lists] extension:

    ```` yaml
    ``` { .yaml .select }
    # Code block content
    ```
    ````

    Note that the language shortcode which has to come first must now also be 
    prefixed by a `.`. Similarly, the selection button can also be disabled for
    a specific code block:

    ```` { .yaml .no-select }
    ``` { .yaml .no-select }
    # Code block content
    ```
    ````

  [Insiders]: ../insiders/index.md
  [line highlighting]: #highlighting-specific-lines

### Code annotations

[:octicons-tag-24: 8.0.0][Code annotations support] ·
:octicons-unlock-24: Feature flag

Code annotations offer a comfortable and friendly way to attach arbitrary
content to specific sections of code blocks by adding numeric markers in block
and inline comments in the language of the code block. Add the following to
`mkdocs.yml` to enable them globally:

``` yaml
theme:
  features:
    - content.code.annotate # (1)!
```

1.  :man_raising_hand: I'm a code annotation! I can contain `code`, __formatted
    text__, images, ... basically anything that can be written in Markdown.

??? info "Enabling code annotations for a specific code block"

    If you don't want to enable code annotations globally, because you don't
    like the automatic inlining behavior, you can enable them for a specific
    code block by using a slightly different syntax based on the
    [Attribute Lists] extension:

    ```` yaml
    ``` { .yaml .annotate }
    # Code block content
    ```
    ````

    Note that the language shortcode which has to come first must now also be 
    prefixed by a `.`.

  [Code annotations support]: https://github.com/squidfunk/mkdocs-material/releases/tag/8.0.0
  [Attribute Lists]: ../setup/extensions/python-markdown.md#attribute-lists

#### Custom selectors :material-alert-decagram:{ .mdx-pulse title="Added on February 19, 2023" }

[:octicons-heart-fill-24:{ .mdx-heart } Sponsors only][Insiders]{ .mdx-insiders } ·
[:octicons-tag-24: insiders-4.32.0][Insiders] ·
:octicons-beaker-24: Experimental

Normally, code annotations can only be [placed in comments], as comments can be
considered safe for placement. However, sometimes it might be necessary to place
annotations in parts of the code block where comments are not allowed, e.g. in 
strings.

Additional selectors can be set per-language:

``` yaml
extra:
  annotate:
    json: [.s2] # (1)!
```

1.  [`.s2`][s2] is the name of the lexeme that [Pygments] generates for double-quoted
    strings. If you want to use a code annotation in another lexeme than a
    comment, inspect the code block and determine which lexeme needs to be added
    to the list of additional selectors.

    __Important__: Code annotations cannot be split between lexemes.

Now, code annotations can be used from within strings in JSON:

``` json
{
  "key": "value (1)"
}
```

1.  :man_raising_hand: I'm a code annotation! I can contain `code`, __formatted
    text__, images, ... basically anything that can be written in Markdown.

  [placed in comments]: #adding-annotations
  [s2]: https://github.com/squidfunk/mkdocs-material/blob/87d5ca487b9d9ab95c41ee72813149d214048693/src/assets/stylesheets/main/extensions/pymdownx/_highlight.scss#L45

## Usage

Code blocks must be enclosed with two separate lines containing three backticks.
To add syntax highlighting to those blocks, add the language shortcode directly
after the opening block. See the [list of available lexers] to find the
shortcode for a given language:

```` markdown title="Code block"
``` py
import tensorflow as tf
```
````

<div class="result" markdown>

``` py
import tensorflow as tf
```

</div>

  [list of available lexers]: https://pygments.org/docs/lexers/

### Adding a title

In order to provide additional context, a custom title can be added to a code
block by using the `title="<custom title>"` option directly after the shortcode,
e.g. to display the name of a file:

```` markdown title="Code block with title"
``` py title="bubble_sort.py"
def bubble_sort(items):
    for i in range(len(items)):
        for j in range(len(items) - 1 - i):
            if items[j] > items[j + 1]:
                items[j], items[j + 1] = items[j + 1], items[j]
```
````

<div class="result" markdown>

``` py title="bubble_sort.py"
def bubble_sort(items):
    for i in range(len(items)):
        for j in range(len(items) - 1 - i):
            if items[j] > items[j + 1]:
                items[j], items[j + 1] = items[j + 1], items[j]
```

</div>

### Adding annotations

Code annotations can be placed anywhere in a code block where a comment for the
language of the block can be placed, e.g. for JavaScript in `#!js // ...` and
`#!js /* ... */`, for YAML in `#!yaml # ...`, etc.[^1]:

  [^1]:
    Code annotations require syntax highlighting with [Pygments] – they're
    currently not compatible with JavaScript syntax highlighters, or languages
    that do not have comments in their grammar. However, we're actively working
    on supporting alternate ways of defining code annotations, allowing to
    always place code annotations at the end of lines.

```` markdown title="Code block with annotation"
``` yaml
theme:
  features:
    - content.code.annotate # (1)
```

1.  :man_raising_hand: I'm a code annotation! I can contain `code`, __formatted
    text__, images, ... basically anything that can be written in Markdown.
````

<div class="result" markdown>

``` yaml
theme:
  features:
    - content.code.annotate # (1)
```

1.  :man_raising_hand: I'm a code annotation! I can contain `code`, __formatted
    text__, images, ... basically anything that can be written in Markdown.

</div>

#### Stripping comments

[:octicons-tag-24: 8.5.0][Stripping comments support] ·
:octicons-beaker-24: Experimental

If you wish to strip the comment characters surrounding a code annotation,
simply add an `!` after the closing parenthesis of the code annotation:

```` markdown title="Code block with annotation, stripped"
``` yaml
# (1)!
```

1.  Look ma, less line noise!
````

<div class="result" markdown>

``` yaml
# (1)!
```

1.  Look ma, less line noise!

</div>

Note that this only allows for a single code annotation to be rendered per
comment. If you want to add multiple code annotations, comments cannot be
stripped for technical reasons.

  [Stripping comments support]: https://github.com/squidfunk/mkdocs-material/releases/tag/8.5.0

### Adding line numbers

Line numbers can be added to a code block by using the `linenums="<start>"`
option directly after the shortcode, whereas `<start>` represents the starting
line number. A code block can start from a line number other than `1`, which
allows to split large code blocks for readability:

```` markdown title="Code block with line numbers"
``` py linenums="1"
def bubble_sort(items):
    for i in range(len(items)):
        for j in range(len(items) - 1 - i):
            if items[j] > items[j + 1]:
                items[j], items[j + 1] = items[j + 1], items[j]
```
````

<div class="result" markdown>

``` py linenums="1"
def bubble_sort(items):
    for i in range(len(items)):
        for j in range(len(items) - 1 - i):
            if items[j] > items[j + 1]:
                items[j], items[j + 1] = items[j + 1], items[j]
```

</div>

### Highlighting specific lines

Specific lines can be highlighted by passing the line numbers to the `hl_lines`
argument placed right after the language shortcode. Note that line counts start
at `1`, regardless of the starting line number specified as part of
[`linenums`][Adding line numbers]:

```` markdown title="Code block with highlighted lines"
``` py hl_lines="2 3"
def bubble_sort(items):
    for i in range(len(items)):
        for j in range(len(items) - 1 - i):
            if items[j] > items[j + 1]:
                items[j], items[j + 1] = items[j + 1], items[j]
```
````

<div class="result" markdown>

``` py linenums="1" hl_lines="2 3"
def bubble_sort(items):
    for i in range(len(items)):
        for j in range(len(items) - 1 - i):
            if items[j] > items[j + 1]:
                items[j], items[j + 1] = items[j + 1], items[j]
```

</div>

  [Adding line numbers]: #adding-line-numbers

### Highlighting inline code blocks

When [InlineHilite] is enabled, syntax highlighting can be applied to inline
code blocks by prefixing them with a shebang, i.e. `#!`, directly followed by
the corresponding [language shortcode][list of available lexers].

``` markdown title="Inline code block"
The `#!python range()` function is used to generate a sequence of numbers.
```

<div class="result" markdown>

The `#!python range()` function is used to generate a sequence of numbers.

</div>

### Embedding external files

When [Snippets] is enabled, content from other files (including source files)
can be embedded by using the [`--8<--` notation][Snippets notation] directly
from within a code block:

```` markdown title="Code block with external content"
``` title=".browserslistrc"
--8<-- ".browserslistrc"
```
````

<div class="result" markdown>

``` title=".browserslistrc"
last 4 years
```

</div>

  [Snippets notation]: https://facelessuser.github.io/pymdown-extensions/extensions/snippets/#snippets-notation

## Customization

### Custom syntax theme

If [Pygments] is used, Material for MkDocs provides the [styles for code blocks]
[colors], which are built with a custom and well-balanced palette that works
equally well for both [color schemes]:

- :material-checkbox-blank-circle:{ style="color: var(--md-code-hl-number-color) " } `--md-code-hl-number-color`
- :material-checkbox-blank-circle:{ style="color: var(--md-code-hl-special-color) " } `--md-code-hl-special-color`
- :material-checkbox-blank-circle:{ style="color: var(--md-code-hl-function-color) " } `--md-code-hl-function-color`
- :material-checkbox-blank-circle:{ style="color: var(--md-code-hl-constant-color) " } `--md-code-hl-constant-color`
- :material-checkbox-blank-circle:{ style="color: var(--md-code-hl-keyword-color) " } `--md-code-hl-keyword-color`
- :material-checkbox-blank-circle:{ style="color: var(--md-code-hl-string-color) " } `--md-code-hl-string-color`
- :material-checkbox-blank-circle:{ style="color: var(--md-code-hl-name-color) " } `--md-code-hl-name-color`
- :material-checkbox-blank-circle:{ style="color: var(--md-code-hl-operator-color) " } `--md-code-hl-operator-color`
- :material-checkbox-blank-circle:{ style="color: var(--md-code-hl-punctuation-color) " } `--md-code-hl-punctuation-color`
- :material-checkbox-blank-circle:{ style="color: var(--md-code-hl-comment-color) " } `--md-code-hl-comment-color`
- :material-checkbox-blank-circle:{ style="color: var(--md-code-hl-generic-color) " } `--md-code-hl-generic-color`
- :material-checkbox-blank-circle:{ style="color: var(--md-code-hl-variable-color) " } `--md-code-hl-variable-color`

Code block foreground, background and line highlight colors are defined via:

- :material-checkbox-blank-circle:{ style="color: var(--md-code-fg-color) " } `--md-code-fg-color`
- :material-checkbox-blank-circle:{ style="color: var(--md-code-bg-color) " } `--md-code-bg-color`
- :material-checkbox-blank-circle:{ style="color: var(--md-code-hl-color) " } `--md-code-hl-color`

Let's say you want to change the color of `#!js "strings"`. While there are
several [types of string tokens], they use the same color. You can assign
a new color by using an [additional style sheet]:

=== ":octicons-file-code-16: `docs/stylesheets/extra.css`"

    ``` css
    :root > * {
      --md-code-hl-string-color: #0FF1CE;
    }
    ```

=== ":octicons-file-code-16: `mkdocs.yml`"

    ``` yaml
    extra_css:
      - stylesheets/extra.css
    ```

If you want to tweak a specific type of string, e.g. ``#!js `backticks` ``, you
can lookup the specific CSS class name in the [syntax theme definition], and
override it as part of your [additional style sheet]:

=== ":octicons-file-code-16: `docs/stylesheets/extra.css`"

    ``` css
    .highlight .sb {
      color: #0FF1CE;
    }
    ```

=== ":octicons-file-code-16: `mkdocs.yml`"

    ``` yaml
    extra_css:
      - stylesheets/extra.css
    ```

  [colors]: https://github.com/squidfunk/mkdocs-material/blob/master/src/assets/stylesheets/main/_colors.scss
  [color schemes]: ../setup/changing-the-colors.md#color-scheme
  [types of string tokens]: https://pygments.org/docs/tokens/#literals
  [additional style sheet]: ../customization.md#additional-css
  [syntax theme definition]: https://github.com/squidfunk/mkdocs-material/blob/master/src/assets/stylesheets/main/extensions/pymdownx/_highlight.scss

### Annotation tooltip width

If you have a lot of content hosted inside your code annotations, it can be a
good idea to increase the width of the tooltip by adding the following as part
of an [additional style sheet]:

=== ":octicons-file-code-16: `docs/stylesheets/extra.css`"

    ``` css
    :root {
      --md-tooltip-width: 600px;
    }
    ```

=== ":octicons-file-code-16: `mkdocs.yml`"

    ``` yaml
    extra_css:
      - stylesheets/extra.css
    ```

This will render annotations with a larger width:

<div style="--md-tooltip-width: 600px;" markdown>

``` yaml
# (1)!
```

1. Muuuuuuuuuuuuuuuch more space for content

</div>

### Annotations with numbers

Prior to [:octicons-tag-24: 8.1.0][code annotation markers], code annotations
were rendered with markers showing the original number as used by the author.
However, for technical reasons code annotation numbers restart each code block,
which might lead to confusion. For this reason, code annotations now render as
`+` signs which are rotated if they're open to denote that clicking them again
will close them.

If you wish to revert to the prior behavior and display code annotation numbers,
you can add an [additional style sheet] and copy and paste the following CSS:

=== ":octicons-file-code-16: `docs/stylesheets/extra.css`"

    ``` css
    .md-typeset .md-annotation__index > ::before {
      content: attr(data-md-annotation-id);
    }
    .md-typeset :focus-within > .md-annotation__index > ::before {
      transform: none;
    }
    ```

=== ":octicons-file-code-16: `mkdocs.yml`"

    ``` yaml
    extra_css:
      - stylesheets/extra.css
    ```

  [code annotation markers]: https://github.com/squidfunk/mkdocs-material/releases/tag/8.1.0