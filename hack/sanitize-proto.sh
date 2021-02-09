#!/bin/bash

grep -lr --include="*.kt" "com.squareup.wire.Grpc*" | xargs rm
