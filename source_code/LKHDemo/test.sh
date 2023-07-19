#!/bin/bash

output_file="content.txt"  # output file
folder_path="."  # root folder

# Iterate through all the files under the folder
find "$folder_path" \( -name "test.sh" -o -type d -name "build" \) -prune -o -type f -print | while read -r file_path; do
    echo "$file_path"
    if [ -f "$file_path" ]; then  # Check if the file exists
        file_name=$(basename "$file_path")  # get file name
        file_content=$(cat "$file_path")  # egt the contents of the file

        # Removes a single-line, multi-line comment
        # file_content=$(sed 's/\/\*.*\*\///' <<< "$file_content")
        # Removes a multi-line comment
        # file_content=$(sed '/\/\*/,/\*\//d' <<< "$file_content")

        # remove blank line
        file_content=$(sed '/^\s*$/d' <<< "$file_content")

        # Output the file name and contents to the output file
        echo "file_name：$file_name" >> "$output_file"
        echo "\`\`\`c" >> "$output_file"
        echo "$file_content" >> "$output_file"
        echo "\`\`\`" >> "$output_file"
        echo "" >> "$output_file"  # add blank line
        echo >> "$output_file"  # add blank line
    fi
done
