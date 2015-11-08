import math

j = 1
# x-axis
a = 2
# z-axis
b = 0.3

#c = a + j * b = M * (math.cos(a) + j * math.sin(a))
a = math.atan(b / a)

#c = veclen(a, b) * (math.cos(vecangle(a, b)) + j * math.sin(vecangle(a, )))

def crot(a, b):
    return (
        a[0] * b + a[1] * -1,
        a[0] + a[1]
    )



rpd = 180 / math.pi
p = (1, 0)
for x in range(0, 6):
    p = crot(p, 1)
    print(p, math.sqrt(p[0] * p[0] + p[1] * p[1]), math.atan2(p[1], p[0]) * rpd)


'''
c = a + j * b

#2.0223 = 2 + j * 0.3
#j = 0.5566

#4.0112 = 4 + j * 0.3

# a + j * b = M * e ** (j * angle)

M = math.sqrt(a * a + b * b)

print('y', a + j * b, M * math.e ** (j * b))
print('z', math.sqrt(a * a + b * b))

# top left angle
angle = math.atan(b / a)

c = I + j * Q
angle = math.atan(Q / I)

print(c, angle, math.pi * 0.25)

print('2', math.e ** (j * angle), math.cos(angle) + j * math.sin(angle))
print('2-pi', math.pi)

print('3-neg', math.e ** (-j * angle), math.cos(angle) - j * math.sin(angle))
'''
