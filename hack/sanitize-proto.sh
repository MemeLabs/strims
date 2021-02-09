#!/bin/bash

#find android/app/src/main/java -type f -name '*Client.kt' | xargs grep 'Service' | xargs rm
grep -lr --include="*.kt" ": Service" | xargs rm
find android/app/src/main/java -type f -name 'Grpc*' | xargs rm
