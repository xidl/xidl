#!/bin/bash

components=''

for file in components/ui/*.tsx; do
    components+=$(basename $file .tsx)
    components+=' '
done

# echo $components
pnpm dlx shadcn@latest add -y -o ${components};
