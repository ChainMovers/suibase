for dir in /proc/*/fd;
do
    echo -n "$dir "; #need a space to get real columns for the sort
    ls $dir 2>/dev/null | wc -l;
done | sort -n -k 2
