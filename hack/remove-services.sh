#!/bin/bash
# shellcheck disable=SC2016
REMOVE_SERVICES='
/^service/ {
	in_service = 1;
}
{
	if (in_service == 0) {
		body = body $0 "\n"
	}
}
/^}/ {
    in_service = 0;
}
END {
  print body
}
'

OUTDIR=$1
shift
ls schema
mkdir -p /tmp/proto
for NEXT in "$@"
do
    NAME=$(basename -- "$NEXT")
	awk "$REMOVE_SERVICES" < "$NEXT" > "$OUTDIR/$NAME"
done
