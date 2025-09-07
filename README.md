# Doké

  

**Doké** is the language of `Dokédex`, a [GDExtension](https://docs.godotengine.org/en/stable/tutorials/scripting/gdextension/index.html) for [Godot Engine](https://godotengine.org/). It provides tools for creating custom DSLs and parsing Markdown files into native Godot resources.

Its design it's about leaving you as free as possible in your language making, while not *requiring* you to write parsers, and forcing gently your language to be easy to edit in markdown editors like [obsidian](https://obsidian.md/).

`Dokédex` itself is a system to really just open an obsidian vault and edit your game's data right from there, with the goal of supporting *Runtime reload* of this data to make a really easy-to-tweak game. And maybe loading from an online instance or something that would allow shared collaboration on a game's design **while testing the game**.

---

## Features

  

- [x] Parse Markdown files into structured Godot resources.

- [x] Create your resource files

- [x] Extend Godot’s resource pipeline with natural-like Domain Specific Languages.

- [ ] Define and integrate your own domain-specific languages (DSLs). (Under development)

---
## The Doké meta-language

  Doké is a **meta-language**, or rather, an **incomplete** language that you specialize for your needs. Doké operates on markdown files, and always does two things :

##### 1. Frontmatter and "Doké" extraction

Doké extracts the frontmatter and the next section delimited by a `---` line. *(It currently needs a frontmatter for its files, otherwise it reads the whole file up until the third `---` line)*

```
---
some_value : 4
---
Your language will be parsed from here, in addition to all the data in the frontmatter.

---
## You can write your wiki entry and anything you want outside these delimiters.

```

##### 1. Making a tree of statements

```
---
some_value : 4
---
This is a statement:
- It has children statements.
  - They have children (etc...)
  - And other children.

It also has siblings.
They can be on multiple lines as long as its a single paragraph.
For statements that are not list-items, they need to end in `:`
to have children:
 1. you can use any list-item syntax
 2. It gets ignored when building the statement
 3. So that 2. statement here would be "It gets ignored when building the statement"
---
## You can write your wiki entry and anything you want outside these delimiters.

```

## The Standard Way

##### Filling in your game's data
For a simple use case, there is a default parser that you can configure with a few different config files, that define a limited but versatile language-making way.

To avoid nested abstractions when explaining this, I will take an example : Your game, like many games before it, might have **Items** in it.

To make an **item**, you need to set some basic data, like `price`,`name`, `description`... 

This is handled in doké via the frontmatter. When making your item Resource class, you can define an  `_apply_doke_frontmatter(fm : Dictionary)` method, that will be called after `_init` when Doké is making the resource.

##### Supporting Composition

We can now make an item with a price and a name, and we could go on to implementing all our items like this... But in most games, an `Item` can work in many different ways. To avoid maintaining a huge 5000 line `Item` class, or the pitfalls of using a big tree of item sub-classes like `SwordItem`, `FireItem` and then wondering what to do of a `FireSwordItem`...

Items often need to have more complex data types, that encapsulate some of their behavior.

For example, your `Sword` would just be an `Item` with a `melee_attack`. A fiery item would maybe have a `FireModifier` in its `modifiers`.

In Doké, you can support this in two steps.

#### 1. Defining your Item files

First, we will make an  `Item.dokeconfig.yaml` file.

for our example, it will look like :

```yaml
root: Item
children:
  - modifiers?: [ItemModifier]
  - melee_attack: ItemAction
rules:
  - for: ItemModifier
    parser: "**/*ItemModifier.dokedef.yaml"
    
  - for: ItemAction
    parser: "**/*ItemAction.dokedef.yaml"

```

`root: Item` tells doké the type of the resource it will be building.

```yaml
children:
  - modifiers?: [ItemModifier]
  - melee_attack: ItemAction
```

This syntax means `Item` will accept an array of `ItemModifiers`, and a single `ItemAction` that defines its melee_attack.

The melee attack here is **required**, for the sake of showing that it is possible. Maybe you like swords so much that everything in your game is a sword ?

```yaml
rules:
  - for: ItemModifier
    parser: "**/*Modifier.dokedef.yaml"
    
  - for: ItemAction
    parser: "**/*ItemAction.dokedef.yaml"
```

This tells doké where to find all the parsers that make the "abstract types" `ItemModifier` and `ItemAction`.

"Abstract types" here just mean that they have in common that they are parsed by the same SentenceParser. All the sentence parsers for an abstract type are joined into a single parser, so that members of an abstract type can share syntax, it also determines which resource goes where in the previous definition, so that our `Item` file doesn't need to know about any `FireModifier` and `WaterModifier`.

Now we can write the `.dokedef.yaml` files.


#### 2. Defining your language

lets now implement the ItemModifier. Our game uses this base class as a way to know how to interact with modifiers, but we have a *(preferably shallow)* hierarchy of Modifiers that implement different effects the item can have.

Some modifiers can add or remove stats. Let's make a `StatModifier.dokedef.yaml`

```yaml
StatModifier:
  - "{op : StatOperator} {amount : int} {stat : Stat} to {target : Target}"
  - "{op : StatOperator} {amount : int} {stat : Stat} from {target : Target}" 
  - "Gives you {amount : int} bonus {stat : Stat}"
  - "Makes you jump incontrollably" : JumpIncontrollablyModifier
    
Stat:
  - health : l"stats/health"
  - max health : l"stats/max_health"
  - swagger : l"stats/swagger"
    
Target:
  - you : 0
  - your pet : 1
    
StatOperator : 
  - "+" : l"+"
  - "-" : l"-"
  - "Adds" : l"+"
  - "Removes" : l"-"

```



Here are examples of valid StatMofidifers now :

`Adds 4 health to you`

`+ 4 health to your pet`

`Gives you 10000 bonus swagger`

The built `Statmodifier` will be a `StatModifier` Resource that looks like :

```yaml
StatModifier {
  op : "+",
  amount : 10000,
  stat : l"stats/health",
  target : 0 # For an enum, for example
}
```

This is not Scribblenauts yet, and there are a few **Caveats**.

notice this line :
```yaml
  - "Makes you jump incontrollably" : JumpIncontrollablyModifier
```

The Right hand side of a rule defaults to the section target, so it was implied that

```yaml
- "{op : StatOperator} (...) from {target : Target}" : StatModifier
```

#### Right hand side

The right hand side of a rule can be:

- A type of resource if it is an identifier in a string (no funny characters) : `StatModifier`
- A string literal, or a formatted string : `l"Literally a string"` `f"you can {format}, yay !"`. Formatted string draw first from the left hand side basic values, then from the frontmatter.
- An int or a float litteral : `2506` `2.5151`

And soon (tm) :
  - A type with "fixed" fields : `StatModifier{op : +, stat : stats/health}`
  - A type with a form of constructor arguments : `StatModifier(+)` which would call `_use_doke_args("+")` when making the resource. (And/Or its constructor directly ?)

This should already allow you to make many things. Here are a few things you also get for free, to soften you up before I talk about the limitations of this approach.

- Templating from the frontmatter : By default, in the actual markdown, you can write {price} and it will get replaced by the value from the frontmatter. I haven't tried, but enabling Mdx support in some editors could make this quite seamless as you would also see that value in the editor preview mode.
- A debug printing parser to see what is going on in the pipe of parsers before the validation step.
# Sentence Parser Limitations

You might be thinking : WOW i'm going to do this :
```yaml
Effect
- "{fct1: Effect}{fct2: Effect}" : DoubleEffect

```
And now I can chain effects !

...

No, this fails horribly. To "chain Effects", you would prefer (when using this "easy" parser style) to do this in Item.dokeconfig.yaml :

```yaml
rules:
  - for: ItemModifier
    parser: "**/*ItemModifier.dokedef.yaml"
    children: [ItemEffect]

```

This will enable a **component-like** structure. You can also use multiple children mappings with names that collect different abstract types.
This currently does not support checking for a single child or all the sugar that is in the root's children. Low priority right now.

```yaml
rules:
  - for: ActionComponent
    parser : "**/*ActionComponent.dokedef.yaml"
    children: 
      - components : [ActionComponent]
      - visuals : [Visuals]


```


Grammars, which would allow this, typically require some care to make, and tooling to make them often absolutely requires that this is done at compile-time. 

The sentence parser, is a recursive bunch of regexps. any non-basic type it encounters in the rule gets captured by a ".+" regex and put away for later to be parsed again by the same set of regexes.

It's quite dumb, and cannot do much complex combinations of things.

I'll be working (low priority for now) on a way to use IPC to allow you to use rules like 

```
"[(amount : Amount, op : Op, thing : Thing)]"
``` 

That would defer the parsing to your own parser and accept arguments, where you could compile your parser as an executable that spits out json when asked about a string, with some type context.

I will also try to use some less-dumb grammars for intermediary steps, to allow you some syntactic sugar in cases like this

```yaml
  - "Deals {damage: int} damage"
  - "Deals {damage: int} damage to {target : Target}"
  - "Deals {damage: int} damage to {target : Target}, with {element : Element}"
  - "Deals {damage: int} damage with {element : Element}"
```

That also would be less heavy than many regexps matching the same kind of things for every combination of missing parameter.

Something like 

```yaml
  - "Deals {damage: int} damage||to {target : Target}|| with {element : Element}||"
```
Or other ideas.

