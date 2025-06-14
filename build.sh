#!/bin/zsh

# Script to build Kolloquy's server and scripts

#########################
###  Compile scripts  ###
#########################

echo "----- Compiling scripts -----"

rm client/dist/*.js

tsc || exit
cd client/dist || exit

for script in *.js; do
  out="${script%.js}.min.js"

  npx uglify-js --validate --keep-fnames -co "$out" "$script"
done

cd ../..

#################
### Packaging ###
#################

echo "----- Packaging -----"

tar -cav --exclude server/target --file=build.tar server/**/* ssl/* ringtones/* client/*.html client/*.handlebars client/*.css client/*.gif client/dist/*.min.js client/icons/* .env