from convex import ConvexClient

client = ConvexClient("https://flippant-cardinal-923.convex.cloud")
mutation_result = client.mutation("sample_mutation:sample", {})
print(client.query("users:list"))
sub = client.subscribe("users:list", {})
