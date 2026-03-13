#!/bin/sh

# Since upgrading to angular 20, the builder tends to duplicate TaskOptionEnumField with another enumeration called TaskOptionEnumField$1.
# It resulted into an impossibility for the application using this package to run.
# Script to remove TaskOptionEnumField$1 declaration and replace all occurrences with TaskOptionEnumField

file="dist/aneoconsultingfr/armonik.api.angular/index.d.ts"

# Remove the enum declaration block for TaskOptionEnumField$1
sed -i '/declare enum TaskOptionEnumField\$1 {/,/}/d' "$file"

# Replace all occurrences of TaskOptionEnumField$1 with TaskOptionEnumField
sed -i 's/TaskOptionEnumField\$1/TaskOptionEnumField/g' "$file"