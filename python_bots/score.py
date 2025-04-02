round1 = [["zev",15750],["zev_m",13750],["zevmcmanusmendelowitz",14750],["ih2427",14750]]
round2 = [["zev",14750],["zev_m",14750],["zevmcmanusmendelowitz",14750],["ih2427",14750]]

final = {}

for id, score in round1:
    final[id] = score

for id, score in round2:
    final[id] = final.get(id, 0) + 2 * score

print(sorted(list(final.items()), key=lambda x: x[1], reverse=True))