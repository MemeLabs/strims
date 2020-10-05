#!/bin/bash
# shellcheck disable=SC2016

OUTDIR=$1
SOURCES=($2)

REMOVE_SERVICES='
{
  if ($1 == "service") {
    in_service = 1;
  }

  if (in_service == 0) {
    body = body $0 "\n"
  }

  if ($1 == "}") {
    in_service = 0;
  }
}
END {
  print body
}
'

for SOURCE in "${SOURCES[@]}"
do
  NAME=$(basename -- "$SOURCE")
	awk "$REMOVE_SERVICES" < "$SOURCE" > "$OUTDIR/$NAME"
done
