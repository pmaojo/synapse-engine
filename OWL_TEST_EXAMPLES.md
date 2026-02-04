# OWL Reasoning Test Examples

## Test 1: Type Propagation (Instance Reasoning)
**Input:**
```
(MyAppleTree, rdf:type, AppleTree)
```

**Expected Inferences:**
- (MyAppleTree, rdf:type, FruitTree)
- (MyAppleTree, rdf:type, Tree)
- (MyAppleTree, rdf:type, Plant)
- (MyAppleTree, rdf:type, LivingOrganism)

## Test 2: Domain/Range Reasoning
**Input:**
```
(MyTree, growsIn, MyGarden)
```

**Expected Inferences:**
- (MyTree, rdf:type, Plant)  # from domain of growsIn
- (MyGarden, rdf:type, AgroEcosystem)  # from range of growsIn

## Test 3: Transitive Property
**Input:**
```
(Leaf, partOf, Branch)
(Branch, partOf, Tree)
```

**Expected Inference:**
- (Leaf, partOf, Tree)  # transitivity

## Test 4: Inverse Property
**Input:**
```
(AppleTree, growsIn, FoodForest)
```

**Expected Inference:**
- (FoodForest, supports, AppleTree)  # inverse

## How to Test in UI
1. Go to Tab 2: "OWL Reasoning"
2. Paste one of the inputs above
3. Click "Apply Reasoning"
4. Check terminal logs for debug output
5. Verify inferred triples > 0
