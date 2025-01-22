raw_message = "8=FIX.4.29=10835=D49=A56=B34=1238=10052=20100318-03:21:11.36411=12321=255=AAPL54=160=20100318-03:21:11.36440=7"

tailPosition = -1
msgForCheckSum = raw_message
sum = 0
for c in msgForCheckSum:
    # If written as pipe, only add SOH (ascii 1)
    if c == "|":
        sum += 1
    else:
        sum += ord(c)
sum = sum % 256
print(sum)