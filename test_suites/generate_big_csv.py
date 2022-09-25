#!/usr/bin/env python3
import argparse

parser = argparse.ArgumentParser(description='Generate a very big CSV file.')
parser.add_argument('--rows', type=int, help='how many rows to generate')
args = parser.parse_args()

print("type,client,tx,amount")

for i in range(0, args.rows):
    client = i % 128
    print(f"deposit,{client},{i},1")
