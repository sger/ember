; FizzBuzz in EMBER

def fizzbuzz
    dup 15 % 0 = ["FizzBuzz" print drop] [
        dup 3 % 0 = ["Fizz" print drop] [
            dup 5 % 0 = ["Buzz" print drop] [
                print
            ] if
        ] if
    ] if
end

1 101 range [fizzbuzz] each
