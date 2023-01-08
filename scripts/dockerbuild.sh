#!/bin/bash

dir="./docker"
[ ! -e $dir ] && echo must be executed from app root directory. && exit 1

command -v jq >/dev/null || {
  echo "jq command isnt available"
  exit 1
}

tmpctx=$dir/ctx
mkdir -p $tmpctx
rsync -vhra ./ $tmpctx/ --include='**.gitignore' --exclude='/.git' --filter=':- .gitignore' --delete-after

pkg="$tmpctx/client/package.json"
[ ! -f "$pkg" ] && echo $pkg isnt available && exit 1
for v in name version; do
  declare "${v}=$(jq -r .${v} $pkg)"
  [ "${!v}" = "null" ] && echo cant read $v && exit 1
done

cd $dir
base=mmta

docker build -f Dockerfile -t $base/$name:$version -t $base/$name:latest .

if [ "$1" = "push" ]; then
  echo pushing
  docker push $base/$name:$version -t $base/$name:latest
fi
