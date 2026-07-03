#! /bin/bash

USE_KEYRING=1 BIND_TO="127.0.0.1:1234" PASS2_PEPPER="PqlwxkhmboQa4An8MMoAiw4uYeWjjR3exSwA1YJL" EXPECT_CWD="$(pwd)" ./kolloquy-phash -T0 --no-filecheck
