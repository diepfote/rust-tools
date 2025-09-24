# Filestile

Just keep one file of many that match a condition
(and yes, this is a pun on turnstile and file).

The tool looks for matching files based on their filename.  
You provide a string such as "Folge 85" or "Episode 85".
It will then run `stat` for each of the files that match
and delete all but the oldest file.

In the end it should also be able to handle not just strings
but regexes. "Folge [0-9]+" just be valid and result in the
following behavior:

* a list of all matching files is generated
* we print this list
* we sort them into files that are "identical" (if we have files that contain "Folge 85" and "Folge 86" we want to sort them into 2 buckets)
* we remove all but the oldest version of each of these files

