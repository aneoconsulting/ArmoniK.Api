#!/bin/sh

sed -E '
          s/# Protocol Documentation/# V1/;
          /## Table of Contents/,/^[^#]*## /{
            /## agent_common.proto/!d  # Exclude ## agent_common.proto from being deleted
          };
          s/name="([^"]*)"/id="\1"/g
        ' .docs/content/api/tmp.md > .docs/content/api/v1.md