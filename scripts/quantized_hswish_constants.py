from fractions import Fraction as F
import math

QA = 255

# hswish |y| = y * (y / 6.0 + 0.5)
# y * (y / 6.0 + 0.5) = 1
# y = (sqrt(33) - 3) / 2

Y = (math.sqrt(33) - 3) / 2

# => hswish |y| = (y * (y+3)) / 6
# => q_hswish |y / 255| = ((y / 255) * (y / 255 + 3)) / 6
# => q_hswish |y| = (y * (y + 3 * 255)) / 6 * (255**2)

QA3 = QA * 3
QA6 = QA * 6

YMAX = round(QA * Y)
GRID = [min(QA, math.floor(F(y * (y + QA3), QA6) + F(1, 2))) for y in range(YMAX + 1)]


def mulhrs(a, b):
    if not (-32768 <= a <= 32767 and -32768 <= b <= 32767):
        raise OverflowError
    return (a * b + 16384) >> 15


def score(f):
    try:
        out = [f(y) for y in range(YMAX + 1)]
    except OverflowError:
        return None
    diffs = sum(o != t for o, t in zip(out, GRID))
    worst = max(abs(o - t) for o, t in zip(out, GRID))
    return diffs, worst


res = []

# q_hswish |y| = mulhrs(mulhrs(y*c1, (y+QA3)*c2), C)
for c1 in range(1, 94):
    for c2 in range(1, 30):
        s = c1 * c2
        C = round(32768 * 32768 / (s * 1530))
        for CC in (C - 1, C, C + 1):
            r = score(lambda y: mulhrs(mulhrs(y * c1, (y + QA3) * c2), CC))

            if r:
                res.append((r[0], r[1], c1, c2, CC))

res.sort()
for s1, s2, c1, c2, cc in res[:10]:
    print(f"C1={c1} C2={c2} CC={cc} diffs={s1} worst={s2} ")
