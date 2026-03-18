# Triple Count

## Endpoint

http://localhost:7000/sparql

## `triple_count` Count all triples

```sparql
SELECT (COUNT(*) AS ?count)
WHERE {
  ?s ?p ?o
}
```

## Output

```javascript
({
  json({triple_count}) {
    const row = triple_count.results.bindings[0];
    return { count: row ? Number(row.count.value) : 0 };
  },

  text({triple_count}) {
    const row = triple_count.results.bindings[0];
    return row ? row.count.value : "0";
  }
})
```
