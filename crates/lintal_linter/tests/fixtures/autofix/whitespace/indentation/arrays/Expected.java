public class Input {
    int[] simpleArray = {
        1,
        2,
        3
    };

    String[] stringArray = {
        "one",
        "two",
        "three"
    };

    int[][] twoDimensional = {
        {
            1, 2, 3
        },
        {
            4, 5, 6
        },
        {
            7, 8, 9
        }
    };

    Object[] mixedArray = {
        new Object(),
        new Object(),
        new Object()
    };

    void methodWithArrays() {
        int[] local = {
            10,
            20,
            30
        };

        String[][] nested = {
            {
                "a", "b"
            },
            {
                "c", "d"
            }
        };
    }

    int[] returnArray() {
        return new int[] {
            100,
            200,
            300
        };
    }

    void arrayParameter(int[] arr) {
        process(new int[] {
            1,
            2,
            3
        });
    }

    void process(int[] arr) {}

    int[][] complexArray = {
        {
            1,
            2,
            3
        },
        {
            4,
            5,
            6
        }
    };
}
