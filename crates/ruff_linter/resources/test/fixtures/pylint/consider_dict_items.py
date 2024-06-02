ORCHESTRA = {
    "violin": "strings",
    "oboe": "woodwind",
    "tuba": "brass",
    "gong": "percussion",
}

# Errors
for instrument in ORCHESTRA:
    print(f"{instrument}: {ORCHESTRA[instrument]}")

for instrument in ORCHESTRA.keys():
    print(f"{instrument}: {ORCHESTRA[instrument]}")

for instrument in (temp_orchestra := {"violin": "strings", "oboe": "woodwind"}):
    print(f"{instrument}: {temp_orchestra[instrument]}")

# Non errors
for instrument, section in ORCHESTRA.items():
    print(f"{instrument}: {section}")

for instrument, section in (temp_orchestra := {"violin": "strings", "oboe": "woodwind"}).items():
    print(f"{instrument}: {section}")
