; stdlib/math.em
; Mathematical functions and constants

module Math

; Constants - these push values directly
def pi 3.14159265359 end
def e 2.71828182846 end

; Basic operations - these are word definitions, not quotations
def square dup * end

def cube dup dup * * end

def double 2 * end

def half 2.0 / end

def triple 3 * end

def quadruple 4 * end

def circle_area dup * pi * end

def circle_circumference 2 * pi * end

def is_even 2 % 0 = end

def is_odd 2 % 1 = end

def sign
    dup 0 >
    [ drop 1 ]
    [ dup 0 <
        [ drop -1 ]
        [ drop 0 ]
        if
    ]
    if
end

end
