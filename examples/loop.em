; Hello World 10 times with index

def hello
    dup                     ; duplicate index
    "Hello World " swap .   ; concat with number
    print                   ; print result
end

1 11 range [hello] each
