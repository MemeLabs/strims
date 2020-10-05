#!/bin/bash

OUT=$1
shift
SOURCES="$@"

REORDER_FILE='
{
  # skip syntax declaration
  if ($1 == "syntax" || $1 == "package") {
    next;
  }

  # merge options
  if ($1 == "option") {
    options[$2] = $4
    next
  }

  # skip internal imports
  if ($1 == "import" && $2 ~ /^"[a-z]+\.proto";$/) {
    next
  }

  # merge imports
  if ($1 == "import") {
    imports[$2] = 1
    next
  }

  # split the remaining declarations into enum and non-enum blocks. nested
  # enums will break this but they break protobuf.js too so there is no point
  # in doing something more robust.

  if ($1 == "enum") {
    in_enum = 1;
  }

  if (in_enum == 0) {
    body = body $0 "\n"
  } else {
    enums = enums $0 "\n"
  }

  if ($1 == "}") {
    in_enum = 0;
  }
}
END {
  print "syntax = \"proto3\";";
  for (opt in options) {
    printf("option %s = %s\n", opt, options[opt]);
  }
  for (dep in imports) {
    printf("import %s\n", dep);
  }
  print body
  print enums
}
'
cat $SOURCES | awk "$REORDER_FILE" > $OUT
