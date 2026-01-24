def gcd
    dup 0 = [
        drop
    ] [
        swap over %
        gcd
    ] if
end

48 18 gcd print   ; => 6
