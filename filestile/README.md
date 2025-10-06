# Filestile

Just keep one file of many that match a condition
(and yes, this is a pun on turnstile and file).

Here is a picture of a stile from <https://www.geograph.org.uk/photo/7669435>:
![](./.images/7669435_28e15cd4_1024x1024-2241378507.jpg)

The tool looks for matching files based on a shared filename.
You specify that shared filename with a regex pattern and match groups.


## What it does

1) It will loop through all files in a given directory (non-recursive)
2) if there is a regex match it will keep track of this file and note down its created_date
3) if it encounters another match for this file it will update the entry if the file is older
4) it will then re-loop through all files and remove any file it did not previously keep track of


# Usage

```text
filestile --dry-run -m 2 3 -e '.*(Blocksberg|Tina).*(Folge [0-9]+).*'  -- "$temp"
```

If a file is named
`Bibi & Tina -  Das sprechende Pferd (Folge 29) _ Hörspiel des Monats - DAS ZWEITPLATZIERTE....m4a`
the the pattern we should use is `.*(Blocksberg|Tina).*(Folge [0-9]+).*`.
Match group indexes should be `2 3`
and the shared_filename will end up being 'Tina Folge 29'

