def double
    dup +
end

5 double print

[ 1 2 + print ] call

def sign
    dup 0 > [
        drop "positive"
    ] [
        dup 0 < [
            drop "negative"
        ] [
            drop "zero"
        ] if
    ] if
end

-5 sign print

module Math

def square
    dup *
end

end

5 Math.square print

{ 1 2 3 4 } print

def gcd
    dup 0 = [
        drop
    ] [
        swap over % gcd
    ] if
end

48 18 gcd print
