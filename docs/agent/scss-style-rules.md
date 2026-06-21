# SCSS Style Rules

- Prefer nesting related selectors inside the owning class in SCSS modules.
- Put responsive overrides inside the class they modify:

```scss
.page {
  display: grid;

  @media (width <= 1280px) {
    grid-template-columns: minmax(0, 1fr);
  }
}
```

- Put child element selectors inside the parent class:

```scss
.metaList {
  display: grid;

  div {
    display: grid;
  }

  dt {
    color: rgb(0 0 0 / 45%);
  }

  dd {
    margin: 0;
  }
}
```

- Put pseudo-classes and state variants near the base selector when it keeps the relationship clear.
- Do not create deep nesting just for shape. Prefer one or two nesting levels; extract a separate class when the selector becomes hard to scan.
