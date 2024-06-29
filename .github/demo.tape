Output "demo.gif"
Set Theme "Tomorrow Night"
Set Width 2000
Set Height 1200
Set Framerate 60
Set PlaybackSpeed 0.5 # Make output 2 times slower

Type "flamelens csvlens.data"
Sleep 0.5s
Enter
Sleep 2s

Down @0.5s 9
Sleep 2s

Type @2s "/"
Type @0.2s "read_record"
Enter
Sleep 5s

Enter
Sleep 5s

Type "q"
Sleep 1s
