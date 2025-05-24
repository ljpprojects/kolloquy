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

tar -cav --exclude '**/._*' --exclude './.idea' --exclude './.git' --exclude 'server/target' --file=build.tar .